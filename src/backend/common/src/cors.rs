use aws_lambda_events::{apigw::ApiGatewayProxyResponse, encodings::Body, http::HeaderMap};

pub fn create_cors_response(status_code: i64, body: Option<String>) -> ApiGatewayProxyResponse {
    let mut headers = HeaderMap::new();

    // Add CORS headers
    headers.insert("Access-Control-Allow-Origin", "*".parse().unwrap());
    headers.insert(
        "Access-Control-Allow-Methods",
        "GET,POST,PUT,DELETE,OPTIONS".parse().unwrap(),
    );
    headers.insert(
        "Access-Control-Allow-Headers",
        "Content-Type,Authorization,X-Amz-Date,X-Api-Key,X-Amz-Security-Token"
            .parse()
            .unwrap(),
    );
    headers.insert("Access-Control-Max-Age", "86400".parse().unwrap());
    headers.insert("Content-Type", "application/json".parse().unwrap());

    ApiGatewayProxyResponse {
        status_code,
        headers,
        multi_value_headers: HeaderMap::new(),
        body: body.map(Body::Text),
        is_base64_encoded: false,
    }
}
