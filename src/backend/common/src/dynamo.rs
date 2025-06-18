use aws_sdk_dynamodb::Client as DynamoDbClient;
use tracing::error;

pub async fn update_job_status_to_success(
    table_name: &str,
    job_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let config = aws_config::load_from_env().await;
    let dynamodb_client = DynamoDbClient::new(&config);

    let pk = format!("JOB-{}", job_id);

    println!("Job {}: Updating DynamoDB status to success", job_id);

    let result = dynamodb_client
        .update_item()
        .table_name(table_name)
        .key("service", aws_sdk_dynamodb::types::AttributeValue::S(pk))
        .key(
            "serviceId",
            aws_sdk_dynamodb::types::AttributeValue::S(job_id.to_string()),
        )
        .update_expression("SET #status = :status")
        .expression_attribute_names("#status", "status")
        .expression_attribute_values(
            ":status",
            aws_sdk_dynamodb::types::AttributeValue::S("success".to_string()),
        )
        .send()
        .await;

    match result {
        Ok(_) => {
            println!(
                "Job {}: Successfully updated DynamoDB status to success",
                job_id
            );
            Ok(())
        }
        Err(e) => {
            error!("Job {}: Failed to update DynamoDB status: {}", job_id, e);
            Err(format!("DynamoDB update failed: {}", e).into())
        }
    }
}
