## Getting started

## Required to deploy

node: https://nodejs.org/en/download
npm: https://www.ramotion.com/blog/how-to-install-npm/
rust: https://www.rust-lang.org/tools/install
cargo: https://doc.rust-lang.org/cargo/getting-started/installation.html

I am using the following for the demo
node: 23.6
npm: 11.4
rust: 1.87
cargo: 1.87

## Good to knows

Infrastructure is deployed via sst v3. If you are deploying with a windows machine it's currently in beta, if you work on a linux/unix machine it should work fine.
https://sst.dev/docs/start/aws/svelte/#serverless

You will need to have your aws credentials configured via the CLI to deploy.

### Bedrock gotchas

Currently I am deploying in `ap-southeast-2` and I require an instance profile on bedrock to infer across region for claude. If you are deploying via a region that doesn't support cross region inference for claude, you will need to update `src/backend/parquet/generate-query/index.rs`, line 127 and 165 with the correct modelId.

## Tools used

- SvelteKit 5: https://svelte.dev/docs/kit/introduction,https://sst.dev/docs/start/aws/svelte/#serverless
- Cloudfront to deploy the website
- SST V3: https://sst.dev/docs/start/aws/svelte/#serverless
- Backend is rust
- Frontend uses Typescript/Javascript, HTML and CSS
- AWS Services used: Lambda, API Gateway, S3, SQS, DynamoDB, Bedrock
- Used Claude 4 to assist with developemnt

## Installing everything

Pull the repo
`npm run install`
`npx sst install`
`cargo build --release` OR `cargo build`. --release takes longer but the code will run quicker.

## how to deploy

Ensure that you have ran the install step above
Ensure that your AWS credentials are configured via the CLI.

**If you want to deploy to a different region change the provider.aws.region field in sst.config.ts on line 13, otherwise it will deploy to ap-southeast-2, if you change it you will need to run cargo build again then deploy**

`npx sst deploy --stage prod`
