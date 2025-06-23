export async function updateContext(
	CORE_API_URL: string,
	context: string,
	job_id: string
): Promise<{ statusCode: number }> {
	const response = await fetch(`${CORE_API_URL}/update-context`, {
		method: 'POST',
		headers: {
			'Content-Type': 'application/json'
		},
		body: JSON.stringify({ context, job_id })
	});

	const body = await response.json();

	if (response.status !== 200) {
		throw new Error(JSON.stringify({ error: body.error, detail: body.detail }));
	}

	return { statusCode: response.status };
}
