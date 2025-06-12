export async function generateResponseFromMessage(
	CORE_API_URL: string,
	message: string,
	parquet_key: string
): Promise<{ statusCode: number; response_message: string }> {
	const response = await fetch(`${CORE_API_URL}/generate-parquet-query`, {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json'
		},
		body: JSON.stringify({ message, parquet_key })
	});

	if (response.status !== 200) {
		throw new Error('wrong');
	}

	const body = await response.json();

	const response_message = body.response_message;

	return { statusCode: response.status, response_message };
}
