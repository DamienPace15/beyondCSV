export async function parseCsvToParquet(
	CORE_API_URL: string,
	payload: { column: string; type: string }[],
	s3Key: string
): Promise<{ statusCode: number }> {
	const response = await fetch(`${CORE_API_URL}/parquet-creation`, {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json'
		},
		body: JSON.stringify({ payload, s3Key })
	});

	return { statusCode: response.status };
}
