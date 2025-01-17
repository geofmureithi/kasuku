use std::ops::Deref;

use futures::executor::block_on;

use crate::KasukuDatabase;

#[must_use]
pub struct Transaction<'a> {
    db: &'a KasukuDatabase,
    exhausted: bool,
}

impl<'a> Transaction<'a> {
    pub async fn new(db: &'a KasukuDatabase) -> Self {
        db.execute("BEGIN;")
            .await
            .expect("Could not start transaction");
        Transaction {
            db,
            exhausted: false,
        }
    }

    pub async fn commit(mut self) {
        self.db
            .execute("COMMIT;")
            .await
            .expect("Could not commit transaction");
        self.exhausted = true;
    }

    pub async fn rollback(mut self) {
        self.db
            .execute("ROLLBACK;")
            .await
            .expect("Could not rollback transaction");
        self.exhausted = true;
    }
}

impl Drop for Transaction<'_> {
    fn drop(&mut self) {
        if !self.exhausted {
            tracing::warn!("Dropping a transaction that is not committed. calling ROLLBACK;");
            block_on(self.db.execute("ROLLBACK")).unwrap();
        }
    }
}

impl Deref for Transaction<'_> {
    type Target = KasukuDatabase;

    fn deref(&self) -> &Self::Target {
        self.db
    }
}
