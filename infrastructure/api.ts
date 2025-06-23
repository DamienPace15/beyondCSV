import { dynamoTable } from './dynamo';
import { s3Bucket } from './storage';

export const apiGateway = new sst.aws.ApiGatewayV1('regionalRestAPI', {
	accessLog: { retention: '1 week' },
	endpoint: { type: 'regional' },
	transform: {
		route: { args: { transform: { integration: { timeoutMilliseconds: 120000 } } } },
		api: {
			name: `rest-${$app.stage}-core-api`
		}
	},
	cors: true
});

const parquetQueue = new sst.aws.Queue(`parqueCreationProcessorQueue`, {
	visibilityTimeout: '500 seconds',
	transform: {
		queue: { name: `${$app.stage}-parque-creation-processor`, receiveWaitTimeSeconds: 20 }
	}
});

apiGateway.route('POST /parquet-creation', {
	handler: './.parquet-creation',
	runtime: 'rust',
	memory: '128 MB',
	logging: { logGroup: `${$app.stage}-create-test-parquet` },
	environment: {
		DYNAMODB_NAME: dynamoTable.name,
		PARQUET_QUEUE_URL: parquetQueue.url
	},
	permissions: [
		{
			actions: ['dynamodb:PutItem'],
			effect: 'allow',
			resources: [dynamoTable.arn]
		},
		{
			actions: ['sqs:SendMessage'],
			effect: 'allow',
			resources: [parquetQueue.arn]
		}
	],
	transform: {
		function: {
			name: `${$app.stage}-create-parquet`
		}
	}
});

const parquetProcessorLambda = new sst.aws.Function(`createParquetProcessor`, {
	handler: './.parquet-creation-processor',
	runtime: 'rust',
	memory: '3008 MB',
	timeout: '500 seconds',
	logging: { logGroup: `${$app.stage}-create-parquet-processor` },
	environment: { S3_UPLOAD_BUCKET_NAME: s3Bucket.name, DYNAMODB_NAME: dynamoTable.name },
	permissions: [
		{
			actions: ['s3:GetObject', 's3:Putobject'],
			effect: 'allow',
			resources: [s3Bucket.arn, s3Bucket.arn.apply((arn) => `${arn}/*`)]
		},
		{
			actions: ['sqs:ReceiveMessage', 'sqs:DeleteMessage', 'sqs:GetQueueAttributes'],
			effect: 'allow',
			resources: [parquetQueue.arn]
		},
		{
			actions: ['dynamodb:UpdateItem'],
			effect: 'allow',
			resources: [dynamoTable.arn]
		}
	],
	transform: {
		function: {
			name: `${$app.stage}-create-parquet-processor`
		}
	}
});

parquetQueue.subscribe(parquetProcessorLambda.arn);

apiGateway.route('POST /generate-parquet-query', {
	handler: './.generate-parquet-query',
	runtime: 'rust',
	memory: '1024 MB',
	timeout: '500 seconds',
	logging: { logGroup: `${$app.stage}-generate-parquet-query` },
	environment: { S3_UPLOAD_BUCKET_NAME: s3Bucket.name, DYNAMODB_NAME: dynamoTable.name },
	permissions: [
		{
			actions: ['s3:GetObject'],
			effect: 'allow',
			resources: [s3Bucket.arn, s3Bucket.arn.apply((arn) => `${arn}/*`)]
		},
		{
			effect: 'allow',
			actions: ['bedrock:*'],
			resources: ['*']
		},
		{
			actions: ['dynamodb:GetItem'],
			effect: 'allow',
			resources: [dynamoTable.arn]
		}
	],
	transform: {
		function: {
			name: `${$app.stage}-generate-parquet-query`
		}
	}
});

apiGateway.route('GET /poll-parquet-status/{job_id}', {
	handler: './.poll-parquet-status',
	runtime: 'rust',
	memory: '128 MB',
	logging: { logGroup: `${$app.stage}-poll-parquet-status` },
	environment: {
		DYNAMODB_NAME: dynamoTable.name
	},
	permissions: [
		{
			actions: ['dynamodb:GetItem'],
			effect: 'allow',
			resources: [dynamoTable.arn]
		}
	],
	transform: {
		function: {
			name: `${$app.stage}-poll-parquet-status`
		}
	}
});

apiGateway.route('POST /update-context', {
	handler: './.update-context',
	runtime: 'rust',
	memory: '128 MB',
	logging: { logGroup: `${$app.stage}-update-context` },
	environment: {
		DYNAMODB_NAME: dynamoTable.name
	},
	permissions: [
		{
			actions: ['dynamodb:UpdateItem'],
			effect: 'allow',
			resources: [dynamoTable.arn]
		}
	],
	transform: {
		function: {
			name: `${$app.stage}-update-context`
		}
	}
});

apiGateway.deploy();

const testProcessor = new sst.aws.Function(`test`, {
	handler: './.test-processor',
	runtime: 'rust',
	memory: '3008 MB',
	timeout: '500 seconds',
	logging: { logGroup: `${$app.stage}-test-parquet-processor` },
	environment: { S3_UPLOAD_BUCKET_NAME: s3Bucket.name, DYNAMODB_NAME: dynamoTable.name },
	permissions: [
		{
			actions: ['s3:GetObject', 's3:Putobject'],
			effect: 'allow',
			resources: [s3Bucket.arn, s3Bucket.arn.apply((arn) => `${arn}/*`)]
		},
		{
			actions: ['dynamodb:UpdateItem'],
			effect: 'allow',
			resources: [dynamoTable.arn]
		}
	],
	transform: {
		function: {
			name: `${$app.stage}-test-parquet-processor`
		}
	}
});
