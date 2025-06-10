export const table = new sst.aws.Dynamo('dynamo', {
	fields: {
		service: 'string',
		serviceId: 'string'
	},
	primaryIndex: { hashKey: 'service', rangeKey: 'serviceId' },
	transform: { table: { name: `${$app.stage}-csv-single-table` } }
});

export const auth = new sst.aws.Dynamo('tempAuth', {
	fields: {
		email: 'string'
	},
	primaryIndex: { hashKey: 'email' },
	transform: { table: { name: `${$app.stage}-csv-auth-table` } }
});
