# AGENTS.md

## Project Overview

Buzz CSV - A natural language query assistant that processes CSV files and enables conversational data querying. Built with SvelteKit 5 frontend and Rust backend, deployed via SST v3 on AWS Lambda.

## Frontend Structure & Conventions

### SvelteKit 5 Architecture

- **Components**: Organized in `src/lib/` with descriptive folder names (PascalCase)
- **Routes**: Follow SvelteKit conventions in `src/routes/`
- **Server Logic**: Use `+layout.server.ts` for global server setup, `+page.server.ts` for page-specific data
- **Modern Svelte**: Use Svelte 5 syntax with runes (`$props()`, `$state()`, `$derived()`)

### Component Organization

```
src/lib/
├── ChatHeader/header.svelte
├── ChatInput/input.svelte
├── ChatMessage/message.svelte
├── Columns/colums.svelte
├── ContextBox/context.svelte
├── DataKey/data-key.svelte
├── Egg/buzzEgg.svelte
├── Sidebar/chatSidebar.svelte
├── TypingIndicator/indicator.svelte
├── Upload/upload.svelte
├── UploadingStatus/status.svelte
└── index.ts
```

### TypeScript Requirements

- **Strict Typing**: All components must use TypeScript interfaces for props
- **Props Pattern**: Use `interface Props { ... }` and `let { ... }: Props = $props()`
- **Type Safety**: Import types from `$types` for SvelteKit route types
- **AWS SDK**: Use proper TypeScript types for AWS service clients

### Styling Conventions

- **Scoped Styles**: Keep component styles within `<style>` blocks
- **CSS Variables**: Use for consistent theming (Amazon Ember font family, AWS color palette)
- **Responsive Design**: Include mobile breakpoints (`@media (max-width: 768px)`)
- **Animations**: Use CSS keyframes for loading states and transitions

### File Naming Patterns

- **Components**: PascalCase folders with kebab-case filenames (`ChatHeader/header.svelte`)
- **Routes**: Follow SvelteKit conventions (`+page.svelte`, `+layout.server.ts`)
- **Utilities**: camelCase TypeScript files (`sendDataToLambda.ts`, `queryData.ts`)

## Backend Structure & Conventions

### Rust Lambda Organization

```
src/backend/
├── common/src/           # Shared library
├── csv/                  # CSV processing lambdas
│   ├── parquet-creation/
│   ├── parquet-creation-processor/
│   └── test-processor/
└── parquet/              # Parquet query lambdas
    ├── generate-query/
    ├── poller/
    └── update-context/
```

### Common Library Usage

- **Location**: `src/backend/common/src/`
- **Modules**: Organize by functionality (cors, dynamo, s3, parquet_creation, etc.)
- **Exports**: Use `pub mod` declarations in `lib.rs`
- **Shared Types**: Define common structs and enums for data interchange

### Lambda Function Structure

```rust
#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()...;
    let handler = service_fn(handler);
    lambda_runtime::run(handler).await?;
    Ok(())
}

async fn handler(event: LambdaEvent<ApiGatewayProxyRequest>) -> Result<ApiGatewayProxyResponse, Error> {
    // CORS handling
    if event.payload.http_method == "OPTIONS" {
        return Ok(create_cors_response(200, None));
    }
    // Main logic
}
```

### Async Patterns & Error Handling

- **Tokio Runtime**: Use `#[tokio::main]` for all lambda functions
- **Error Types**: Return `Result<T, Error>` from lambda_runtime
- **AWS SDK**: Use async client methods with `.await?`
- **Environment Variables**: Use `env::var()` with proper error handling

### AWS SDK Integration Patterns

- **Configuration**: `aws_config::load_defaults(BehaviorVersion::latest()).await`
- **Clients**: Create clients once per lambda invocation
- **Service Integration**: Use proper service-specific clients (DynamoClient, SqsClient, etc.)
- **Error Mapping**: Convert AWS errors to lambda_runtime::Error

## Infrastructure & Deployment

### SST v3 Configuration

- **Config File**: `sst.config.ts` with TypeScript
- **Stage Management**: Use `input?.stage` for environment-specific behavior
- **Resource Linking**: Link SvelteKit with AWS resources via `link` property
- **Environment Variables**: Pass via `environment` property in SvelteKit config

### Environment Variable Handling

- **Frontend**: Access via `process.env.PRIVATE_*` variables
- **Backend**: Use `env::var()` in Rust lambdas
- **Naming**: Use `PRIVATE_` prefix for sensitive frontend variables
- **Type Safety**: Use non-null assertion operator (`!`) when variables are guaranteed

### AWS Resource Patterns

- **S3**: Use for file storage (CSV uploads, Parquet files)
- **DynamoDB**: Store job status and metadata
- **SQS**: Asynchronous processing queues
- **API Gateway**: Lambda function endpoints
- **Lambda**: Rust binary deployments

## Code Style & Quality

### ESLint/Prettier Configuration

```json
{
	"useTabs": true,
	"singleQuote": true,
	"trailingComma": "none",
	"printWidth": 100
}
```

### Rust Formatting

- **rustfmt**: Use standard rustfmt configuration
- **Cargo.toml**: Organize dependencies logically
- **Binary Targets**: Define all lambda functions as `[[bin]]` sections
- **Release Profile**: Use optimized settings for production (`lto = true`, `codegen-units = 1`)

### Code Quality Standards

- **TypeScript**: Strict mode enabled, no implicit any
- **Rust**: Use `clippy` for linting, avoid `unwrap()` in production
- **Error Handling**: Proper error propagation and user-friendly error messages
- **Logging**: Use structured logging with tracing

## Testing & Build Commands

### Frontend Commands

```bash
npm run dev          # Development server
npm run build        # Production build
npm run check        # Type checking
npm run lint         # ESLint checking
npm run format       # Prettier formatting
npm run preview      # Preview production build
```

### Rust Commands

```bash
cargo build          # Debug build
cargo build --release  # Production build
cargo test           # Run tests
cargo clippy         # Linting
cargo fmt            # Formatting
```

### Deployment Commands

```bash
npx sst install      # Install SST dependencies
npx sst deploy --stage prod  # Deploy to production
npx sst diff         # Show deployment changes
```

## File Organization Rules

### New Frontend Components

1. Create PascalCase folder in `src/lib/`
2. Use kebab-case for `.svelte` filename
3. Export from `index.ts` if needed globally
4. Include TypeScript interface for props
5. Add responsive styles and loading states

### New Lambda Functions

1. Create folder in appropriate `src/backend/` subdirectory
2. Add binary target to `Cargo.toml`
3. Follow established handler pattern
4. Include proper error handling and CORS
5. Add shared types to common library if needed

### Import/Export Patterns

- **Frontend**: Use ES6 imports/exports, barrel exports in `index.ts`
- **Rust**: Use `pub mod` for module organization, `pub use` for re-exports
- **Shared Types**: Define in common library, import across services
- **AWS Services**: Import specific clients, avoid wildcard imports

### Database Patterns

- **DynamoDB**: Use consistent naming for table attributes
- **Job Status**: Follow established status values (pending, success, failed)
- **Context Storage**: Use JSON strings for flexible context data
- **Schema Handling**: Store schema as HashMap<String, String>

### API Response Patterns

- **CORS**: Use `create_cors_response` helper for all responses
- **Error Responses**: Return meaningful error messages with proper HTTP status codes
- **Success Responses**: Use JSON format with consistent structure
- **Validation**: Validate request bodies before processing

## Security Considerations

### Input Validation

- **CSV Files**: Validate file format and size before processing
- **User Input**: Sanitize natural language queries before processing
- **Schema Inference**: Validate inferred schemas before storage

### AWS Security

- **IAM Roles**: Use least-privilege access for lambda functions
- **Environment Variables**: Never commit secrets to version control
- **Cross-Region**: Handle Bedrock cross-region inference properly
- **Resource Access**: Validate resource access before operations

This structure ensures consistency across the codebase while maintaining scalability and security for the Buzz CSV application.
