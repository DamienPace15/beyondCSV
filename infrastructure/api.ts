import { s3Bucket } from './storage';

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

apiGateway.route('POST /parquet-creation', {
	handler: './.parquet-creation',
	runtime: 'rust',
	memory: '128 MB',
	timeout: '500 seconds',
	logging: { logGroup: `${$app.stage}-create-parquet` },
	environment: { S3_UPLOAD_BUCKET_NAME: s3Bucket.name },
	permissions: [
		{
			actions: ['s3:GetObject', 's3:Putobject'],
			effect: 'allow',
			resources: [s3Bucket.arn, s3Bucket.arn.apply((arn) => `${arn}/*`)]
		}
	],
	transform: {
		function: {
			name: `${$app.stage}-create-parquet`
		}
	}
});

apiGateway.route('POST /generate-parquet-query', {
	handler: './.generate-parquet-query',
	runtime: 'rust',
	memory: '128 MB',
	timeout: '500 seconds',
	logging: { logGroup: `${$app.stage}-generate-parquet-query` },
	environment: { S3_UPLOAD_BUCKET_NAME: s3Bucket.name },
	permissions: [
		{
			actions: ['s3:GetObject'],
			effect: 'allow',
			resources: [s3Bucket.arn, s3Bucket.arn.apply((arn) => `${arn}/*`)]
		}
	],
	transform: {
		function: {
			name: `${$app.stage}-generate-parquet-query`
		}
	}
});
