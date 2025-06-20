export async function generateResponseFromMessage(
	CORE_API_URL: string,
	message: string,
	parquet_key: string,
	job_id: string
): Promise<{ statusCode: number; response_message: string }> {
	const response = await fetch(`${CORE_API_URL}/generate-parquet-query`, {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json'
		},
		body: JSON.stringify({ message, parquet_key, job_id })
	});

	const body = await response.json();

	if (response.status !== 200) {
		throw new Error(JSON.stringify({ error: body.error, detail: body.detail }));
	}

	const response_message = body.response_message;

	return { statusCode: response.status, response_message };
}

export async function pollStatus(
	CORE_API_URL: string,
	job_id: string
): Promise<{ statusCode: number; parquet_complete: boolean }> {
	const response = await fetch(`${CORE_API_URL}/poll-parquet-status/${job_id}`, {
		method: 'GET',
		headers: {
			'Content-Type': 'application/json'
		}
	});

	const body = await response.json();

	console.log(JSON.stringify(body));

	const parquet_complete = body.parquet_complete;

	return { statusCode: response.status, parquet_complete };
}
