import type { LayoutServerLoad } from './$types';

export const load: LayoutServerLoad = async () => {
	return {
		env: {
			CORE_API_URL: process.env.PRIVATE_CORE_API_URL!
		}
	};
};
