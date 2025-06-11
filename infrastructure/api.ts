export const apiGateway = new sst.aws.ApiGatewayV2('easyCSV', {
	accessLog: { retention: '1 week' },
	transform: {
		stage: { autoDeploy: true },
		api: {
			name: `${$app.stage}-core-api`,
			corsConfiguration: {
				allowCredentials: false,
				allowHeaders: ['Content-Type', 'Authorization'],
				allowMethods: ['GET', 'POST', 'PUT', 'DELETE', 'OPTIONS'],
				allowOrigins: ['*'],
				maxAge: 86400
			},
			protocolType: 'HTTP'
		}
	}
});

apiGateway.route('GET /content/{userId}', {
	handler: './.get_headers',
	runtime: 'rust',
	memory: '128 MB',
	logging: { logGroup: `${$app.stage}-get-csv-headers` },
	transform: {
		function: {
			name: `${$app.stage}-get-csv-headers`
		}
	}
});
