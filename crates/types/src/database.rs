use crate::PluginData;

pub struct Database;

#[plugy::macros::context(data = PluginData)]
impl Database {
    pub async fn query(
        caller: &mut plugy::runtime::Caller<'_, plugy::runtime::Plugin<PluginData>>,
        query: String,
    ) -> Vec<u8> {
        bincode::serialize("test").unwrap()
        // let mut storage = caller.data();
        // match storage {
        //     Some(s) => {
        //         let storage = &s.plugin.data.storage;
        //         let fut = send_query(&storage, query).await;
        //         // let res = fut.await.unwrap();
        //         vec![]
        //     }
        //     None => todo!(),
        // }
    }
}
