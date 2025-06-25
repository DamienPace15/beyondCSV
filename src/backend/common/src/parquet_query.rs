use aws_sdk_bedrockruntime::operation::converse::ConverseOutput;
use lambda_runtime::Error;

pub fn get_converse_output_text(output: ConverseOutput) -> Result<String, Error> {
    let text = output
        .output()
        .ok_or("no output")?
        .as_message()
        .map_err(|_| "output not a message")?
        .content()
        .first()
        .ok_or("no content in message")?
        .as_text()
        .map_err(|_| "content is not text")?
        .to_string();
    Ok(text)
}
