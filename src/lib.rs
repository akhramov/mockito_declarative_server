pub use mockito;
pub use serde::Deserialize;
pub use serde_yaml;

#[derive(Deserialize)]
pub struct MockDefinition {
    pub request: MockRequest,
    pub response: MockResponse,
}

#[derive(Deserialize)]
pub struct MockRequest {
    pub method: String,
    pub path: Option<String>,
    pub query: Option<Vec<MockQuery>>,
    pub headers: Option<Vec<MockHeader>>,
}

#[derive(Deserialize)]
pub struct MockQuery {
    pub parameter: String,
    pub value: String,
}

#[derive(Deserialize)]
pub struct MockHeader {
    pub header: String,
    pub value: String,
}

#[derive(Deserialize)]
pub struct MockResponse {
    pub headers: Option<Vec<MockHeader>>,
    pub body: Option<String>,
}

/// Generate mockito mocks using declarative (yml) definition.
#[macro_export]
macro_rules! mock_server {
    ($file:expr) => {{
        use std::path::Path;
        use $crate::mockito::{mock, Matcher};
        use $crate::serde_yaml;
        use $crate::*;

        let mocks_path = Path::new(env!("CARGO_MANIFEST_DIR")).join($file);

        let file = std::fs::read(&mocks_path).expect("definition file's not found!");

        let definitions: Vec<MockDefinition> =
            serde_yaml::from_slice(&file).expect("failed to parse definition file");

        let mocks = definitions
            .into_iter()
            .map(|MockDefinition { request, response }| {
                let path = request.path.map(Matcher::Regex).unwrap_or(Matcher::Any);

                let mut mock = mock(&request.method, path);

                if let Some(headers) = request.headers {
                    for MockHeader { header, value } in headers {
                        mock = mock.match_header(&header, &value[..]);
                    }
                }

                if let Some(query_params) = request.query {
                    for MockQuery { parameter, value } in query_params {
                        mock = mock.match_query(Matcher::UrlEncoded(parameter.into(), value.into()));
                    }
                }

                if let Some(body) = response.body {
                    mock = mock.with_body_from_file(
                        mocks_path
                            .parent()
                            .expect("couldn't extract file parent")
                            .join(body),
                    );
                }

                if let Some(headers) = response.headers {
                    for MockHeader { header, value } in headers {
                        let value = value.replace("SERVER_URL", &mockito::server_url());

                        mock = mock.with_header(&header, &value[..]);
                    }
                }

                mock.create()
            })
            .collect::<Vec<_>>();

        (mockito::server_url(), mocks)
    }};
}

#[cfg(test)]
mod test {
    #[tokio::test]
    async fn test_regex_matching() {
        let (server, _mocks) = super::mock_server!("test/basic.yml");

        let client = reqwest::Client::new();

        let resp = client.get(format!("{}/{}", server, "v2/hash/manifests/hash/"))
            .header("Accept", "application/vnd.docker.distribution.manifest.v2+json")
            .send()
            .await
            .expect("failed to make a request")
            .json::<serde_json::Value>()
            .await
            .expect("failed to parse the response");

        assert_eq!(resp["config"]["size"], 6668);
    }
}
