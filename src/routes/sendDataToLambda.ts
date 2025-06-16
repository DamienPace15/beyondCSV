export async function parseCsvToParquet(
	CORE_API_URL: string,
	payload: { column: string; type: string }[],
	s3_key: string,
	job_id: string
): Promise<{ statusCode: number; parquet_key: string }> {
	console.log('WHAT IS S3', s3_key);
	const response = await fetch(`${CORE_API_URL}/parquet-creation`, {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json'
		},
		body: JSON.stringify({ payload, s3_key, job_id })
	});

	if (response.status !== 200) {
		throw new Error('wrong');
	}

	const body = await response.json();

	const parquet_key = body.parquet_key;

	return { statusCode: response.status, parquet_key };
}
