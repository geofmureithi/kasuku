use serde::de;
use serde::ser::SerializeMap;
use serde::Deserialize;
use serde::Serialize;
use serde::Serializer;
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub vaults: BTreeMap<String, VaultConfig>,
    pub events: BTreeMap<String, Vec<String>>,
    pub internals: Internals,
    #[serde(
        deserialize_with = "deserialize_plugins",
        serialize_with = "serialize_plugins"
    )]
    pub plugins: Vec<PluginConfig>,

    #[serde(default)]
    pub server: ServerConfig,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ServerConfig {
    pub ip: String,
    pub port: u16,
}

impl Default for ServerConfig {
    fn default() -> Self {
        ServerConfig {
            ip: "127.0.0.1".to_owned(),
            port: 3000,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct InnerPlugin {
    pub uri: String,
}

fn deserialize_plugins<'de, D>(deserializer: D) -> Result<Vec<PluginConfig>, D::Error>
where
    D: de::Deserializer<'de>,
{
    struct PluginVisitor;

    impl<'de> de::Visitor<'de> for PluginVisitor {
        type Value = Vec<PluginConfig>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("not a valid plugin")
        }

        fn visit_map<A>(self, mut map: A) -> Result<Self::Value, A::Error>
        where
            A: de::MapAccess<'de>,
        {
            let mut plugins = Vec::new();
            while let Some((name, value)) = map.next_entry()? {
                let InnerPlugin { uri } = value;
                plugins.push(PluginConfig { name, uri });
            }
            Ok(plugins)
        }
    }

    // use our visitor to deserialize an `ActualValue`
    deserializer.deserialize_any(PluginVisitor)
}

// Custom serialization method to mirror `deserialize_plugins`.
pub fn serialize_plugins<S>(plugins: &[PluginConfig], serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    // We'll serialize as a map: { name -> { uri } }
    let mut map = serializer.serialize_map(Some(plugins.len()))?;
    for plugin in plugins {
        // Build the InnerPlugin each time
        let inner = InnerPlugin {
            uri: plugin.uri.clone(),
        };

        // Map key = plugin.name, value = inner
        map.serialize_entry(&plugin.name, &inner)?;
    }
    map.end()
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct VaultConfig {
    pub mount: PathBuf,
    pub plugins: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Internals {
    #[serde(rename = "cache-path")]
    pub database_path: PathBuf,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PluginConfig {
    pub name: String,
    pub uri: String,
}

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RemoteConfig {
    pub friendly_name: String,
    pub description: String,
    pub icon: String,
    pub documentation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventSource {
    Cron(String),
    Plugin(String),
}
