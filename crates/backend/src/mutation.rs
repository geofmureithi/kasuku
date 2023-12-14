use async_graphql::*;

#[derive(InputObject)]
pub struct Command {
    id: String,
    name: String,
}

pub struct MutationRoot;

#[Object]
impl MutationRoot {
    async fn command(&self, _ctx: &Context<'_>, _cmd: Command) -> i32 {
        // let mut books = ctx.data_unchecked::<Storage>().lock().await;
        // let entry = books.vacant_entry();
        // let id: ID = entry.key().into();
        // let book = Book {
        //     id: id.clone(),
        //     name,
        //     author,
        // };
        // entry.insert(book);
        // SimpleBroker::publish(BookChanged {
        //     mutation_type: MutationType::Created,
        //     id: id.clone(),
        // });
        9
        // id
    }

    async fn create_file(&self, _ctx: &Context<'_>, _path: String) -> Result<bool> {
        // let mut books = ctx.data_unchecked::<Storage>().lock().await;
        // let id = id.parse::<usize>()?;
        // if books.contains(id) {
        //     books.remove(id);
        //     SimpleBroker::publish(BookChanged {
        //         mutation_type: MutationType::Deleted,
        //         id: id.into(),
        //     });
        //     Ok(true)
        // } else {

        // }
        Ok(false)
    }

    async fn update_file(&self, _ctx: &Context<'_>, _path: String) -> Result<bool> {
        // let mut books = ctx.data_unchecked::<Storage>().lock().await;
        // let id = id.parse::<usize>()?;
        // if books.contains(id) {
        //     books.remove(id);
        //     SimpleBroker::publish(BookChanged {
        //         mutation_type: MutationType::Deleted,
        //         id: id.into(),
        //     });
        //     Ok(true)
        // } else {
        //     Ok(false)
        // }
        Ok(false)
    }

    async fn delete_file(&self, _ctx: &Context<'_>, _path: String) -> Result<bool> {
        // let mut books = ctx.data_unchecked::<Storage>().lock().await;
        // let id = id.parse::<usize>()?;
        // if books.contains(id) {
        //     books.remove(id);
        //     SimpleBroker::publish(BookChanged {
        //         mutation_type: MutationType::Deleted,
        //         id: id.into(),
        //     });
        //     Ok(true)
        // } else {
        //     Ok(false)
        // }
        Ok(false)
    }
}
