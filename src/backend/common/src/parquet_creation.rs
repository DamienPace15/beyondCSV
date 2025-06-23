use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::{Client as DynamoClient, Error as DynamoError};
use std::collections::HashMap;

pub async fn put_job_status(
    dynamo_client: &DynamoClient,
    table_name: &str,
    service: &str,
    service_id: &str,
    status: &str,
    context: &str,
    schema: &HashMap<String, String>,
) -> Result<(), DynamoError> {
    let mut item = HashMap::new();

    item.insert(
        "service".to_string(),
        AttributeValue::S(service.to_string()),
    );
    item.insert(
        "serviceId".to_string(),
        AttributeValue::S(service_id.to_string()),
    );
    item.insert("status".to_string(), AttributeValue::S(status.to_string()));
    item.insert(
        "context".to_string(),
        AttributeValue::S(context.to_string()),
    );
    let schema_map: HashMap<String, AttributeValue> = schema
        .iter()
        .map(|(k, v)| (k.clone(), AttributeValue::S(v.clone())))
        .collect();
    item.insert("schema".to_string(), AttributeValue::M(schema_map));

    dynamo_client
        .put_item()
        .table_name(table_name)
        .set_item(Some(item))
        .send()
        .await?;

    Ok(())
}
