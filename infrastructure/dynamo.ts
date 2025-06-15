export const dynamoTable = new sst.aws.Dynamo('dynamo', {
	fields: {
		service: 'string',
		serviceId: 'string'
	},
	primaryIndex: { hashKey: 'service', rangeKey: 'serviceId' },
	transform: { table: { name: `${$app.stage}-csv-single-table` } }
});
