
pub struct SubscriptionRoot;

#[Subscription]
impl SubscriptionRoot {
    async fn events(
        &self,
        #[graphql(default = 1)] n: i32,
    ) -> impl futures_util::Stream<Item = i32> {
        let mut value = 0;
        async_stream::stream! {
            loop {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                value += n;
                tracing::debug!("send value");
                yield value
            }
        }
    }
}

async fn graphql_handler_query(
    schema: Extension<schema::ExampleSchema>,
    shutdown: Extension<Shutdown>,
    Query(req): Query<graphql_query::GraphQLQuery>,
) -> axum::response::Response {
    if req.in_query() {
        return GraphQLResponse::from(schema.execute(req).await).into_response();
    }

    // if subscription
    {
        let (abort_handle, abort_registration) = AbortHandle::new_pair();
        let handle = tokio::spawn(async move {
            shutdown._notified().await;
            abort_handle.abort()
        });

        let mut subscription = schema.execute_stream(req);

        let stream: async_graphql::async_stream::AsyncStream<Result<Event, serde_json::Error>, _> = try_stream! {
            while let Some(response) = subscription.next().await {
                let event = Event::default().json_data(response);
                if event.is_err() {
                    handle.abort();
                }
                yield event?;
            }
            handle.abort()
        };

        let stream = Abortable::new(stream, abort_registration);

        Sse::new(stream).into_response()
    }
}