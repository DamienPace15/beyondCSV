use aws_sdk_dynamodb::types::AttributeValue;
use aws_sdk_dynamodb::{Client as DynamoDbClient, Error};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tracing::error;

#[derive(Debug, Serialize, Deserialize)]
pub struct Job {
    pub service: String,
    pub serviceid: String,
    pub status: String,
    pub context: String,
}

impl Job {
    pub fn from_dynamodb_item(
        item: HashMap<String, AttributeValue>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let service = item
            .get("service")
            .and_then(|v| v.as_s().ok())
            .ok_or("Missing or invalid 'service' field")?
            .clone();

        let serviceid = item
            .get("serviceId")
            .and_then(|v| v.as_s().ok())
            .ok_or("Missing or invalid 'serviceid' field")?
            .clone();

        let status = item
            .get("status")
            .and_then(|v| v.as_s().ok())
            .ok_or("Missing or invalid 'status' field")?
            .clone();

        let context = item
            .get("context")
            .and_then(|v| v.as_s().ok())
            .ok_or("Missing or invalid 'context' field")?
            .clone();

        Ok(Job {
            service,
            serviceid,
            status,
            context,
        })
    }
}

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

pub async fn get_job_by_id(table_name: &str, job_id: &str) -> Result<Option<Job>, Error> {
    let config = aws_config::load_from_env().await;
    let dynamodb_client = DynamoDbClient::new(&config);
    let pk_value = format!("JOB-{}", job_id);

    let request = dynamodb_client
        .get_item()
        .table_name(table_name)
        .key("service", AttributeValue::S(pk_value))
        .key("serviceId", AttributeValue::S(job_id.to_string()));

    let response = request.send().await?;

    match response.item {
        Some(item) => match Job::from_dynamodb_item(item) {
            Ok(job) => Ok(Some(job)),
            Err(e) => {
                eprintln!("Error parsing job from DynamoDB item: {}", e);
                Ok(None)
            }
        },
        None => Ok(None),
    }
}
