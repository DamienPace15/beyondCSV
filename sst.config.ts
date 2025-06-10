// eslint-disable-next-line @typescript-eslint/triple-slash-reference
/// <reference path="./.sst/platform/config.d.ts" />

export default $config({
	app(input) {
		return {
			name: 'beyondcsv',
			removal: input?.stage === 'production' ? 'retain' : 'remove',
			protect: ['production'].includes(input?.stage),
			home: 'aws',
			providers: {
				aws: {
					region: 'ap-southeast-2'
				}
			}
		};
	},
	async run() {
		const storage = await import('./infrastructure/storage.ts');
		const dynamo = await import('./infrastructure/dynamo.ts');
		const coreApi = await import('./infrastructure/api.ts');

		new sst.aws.SvelteKit('easyCSVFe', {
			link: [coreApi],
			environment: { PRIVATE_CORE_API_URL: coreApi.apiGateway.url }
		});
	}
});
