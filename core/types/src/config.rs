use serde::de;
use serde::Deserialize;
use serde::Serialize;
use std::collections::BTreeMap;
use std::fmt;
use std::path::PathBuf;

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Config {
    pub vaults: BTreeMap<String, VaultConfig>,
    pub events: BTreeMap<String, Vec<String>>,
    pub internals: Internals,
    #[serde(deserialize_with = "deserialize_plugins")]
    pub plugins: Vec<PluginConfig>,
}

fn deserialize_plugins<'de, D>(deserializer: D) -> Result<Vec<PluginConfig>, D::Error>
where
    D: de::Deserializer<'de>,
{
    struct PluginVisitor;

    #[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
    struct InnerPlugin {
        #[serde(skip_serializing_if = "Option::is_none")]
        pub headless: Option<bool>,
        pub uri: String,
    }

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
                let InnerPlugin { headless, uri } = value;
                plugins.push(PluginConfig {
                    name,
                    headless: headless.unwrap_or_default(),
                    uri,
                    // This will come from remote source
                    remote: None,
                });
            }
            Ok(plugins)
        }
    }

    // use our visitor to deserialize an `ActualValue`
    deserializer.deserialize_any(PluginVisitor)
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
    pub headless: bool,
    pub uri: String,
    pub remote: Option<RemoteConfig>,
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
