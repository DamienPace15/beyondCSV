import { PutObjectCommand, S3Client } from '@aws-sdk/client-s3';
import { getSignedUrl } from '@aws-sdk/s3-request-presigner';
import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = async () => {
	const key = `csvUpload/${crypto.randomUUID()}.csv`;
	const command = new PutObjectCommand({
		Key: key,
		Bucket: process.env.PRIVATE_S3_BUCKET_NAME!
	});

	const url = await getSignedUrl(new S3Client({}), command);

	return {
		env: {
			CORE_API_URL: process.env.PRIVATE_CORE_API_URL!,
			S3_BUCKET_NAME: process.env.PRIVATE_S3_BUCKET_NAME,
			PRESIGNED_URL: url,
			KEY: key
		}
	};
};
