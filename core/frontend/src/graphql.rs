use gloo_net::http::Request;
use serde::{Deserialize, Serialize};
#[derive(Serialize)]
struct GraphQLQuery {
    query: String,
    variables: Variables,
}

#[derive(Serialize)]
struct Variables {
    path: String,
    renderer: Option<String>,
}

#[derive(Deserialize)]
struct GraphQLResponse<T> {
    data: T,
    errors: Option<Vec<GraphQLError>>,
}

#[derive(Deserialize)]
struct GraphQLError {
    message: String,
    // other fields...
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderFileResponse {
    pub render_file: String,
}

pub async fn render_file(
    path: String,
    renderer: Option<String>,
) -> Result<RenderFileResponse, String> {
    let query = GraphQLQuery {
        query: "query RenderFile($path: String!, $renderer: String) { renderFile(path: $path, renderer: $renderer) }".to_string(),
        variables: Variables { path, renderer },
    };
    let response = Request::post("/api/v1/graphql")
        .json(&query)
        .expect("Failed to build request")
        .send()
        .await
        .map_err(|err| err.to_string())?;

    if !response.ok() {
        return Err("Network error".to_string());
    }

    let response_body: GraphQLResponse<RenderFileResponse> =
        response.json().await.map_err(|err| err.to_string())?;

    if let Some(errors) = response_body.errors {
        return Err(errors[0].message.clone());
    }

    Ok(response_body.data)
}
