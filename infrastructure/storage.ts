export const s3Bucket = new sst.aws.Bucket('csvUpload', {
	transform: { bucket: { bucket: `${$app.stage}-csv-upload` } }
});

export const s3TableBucket = new aws.s3tables.TableBucket('parquetBucket', {
	name: `${$app.name}-parquet-bucket`
});
