export const s3Bucket = new sst.aws.Bucket('csvUpload', {
	transform: { bucket: { bucket: `${$app.stage}-csv-upload` } }
});
