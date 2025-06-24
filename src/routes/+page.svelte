<script lang="ts">
	import type { LayoutData } from './$types';
	import { parseCsvToParquet } from './sendDataToLambda';
	import { goto } from '$app/navigation';

	import Colums from '../lib/Columns/colums.svelte';
	import Upload from '../lib/Upload/upload.svelte';
	import UploadingStatus from '../lib/UploadingStatus/status.svelte';
	import ContextBox from '../lib/ContextBox/context.svelte';

	let { data }: { data: LayoutData } = $props();

	// Remove file-related state since Upload component handles it
	let selectedFile = $state<File | null>(null);
	let uploading = $state(false);
	let uploadProgress = $state(0);
	let uploadStatus = $state('');
	let error = $state('');
	let csvHeaders = $state<string[]>([]);
	let csvData = $state<any[][]>([]); // Add CSV data state
	let columnTypes = $state<{ [key: string]: string }>({});
	let excludedColumns = $state<Set<string>>(new Set());
	let contextText = $state('');

	const presignedUrl = data.env.PRESIGNED_URL;
	const key = data.env.key;
	const job_id = data.env.job_id!;

	function initializeColumnTypes(headers: string[]) {
		const types: { [key: string]: string } = {};
		headers.forEach((header) => {
			types[header] = 'string';
		});
		columnTypes = types;
		excludedColumns = new Set();
	}

	// Handle file selection from Upload component
	function handleFileSelect(file: File) {
		selectedFile = file;
		error = '';
		uploadStatus = '';
	}

	// Handle headers read from Upload component
	function handleHeadersRead(headers: string[]) {
		csvHeaders = headers;
		initializeColumnTypes(headers);
	}

	// NEW: Handle CSV data read from Upload component
	function handleDataRead(data: any[][]) {
		csvData = data;
	}

	// Handle errors from Upload component
	function handleUploadError(errorMessage: string) {
		error = errorMessage;
	}

	// Handle reset from Upload component
	function handleReset() {
		selectedFile = null;
		csvHeaders = [];
		csvData = []; // Reset CSV data
		columnTypes = {};
		excludedColumns = new Set();
		contextText = '';
		error = '';
		uploadStatus = '';
		uploadProgress = 0;
	}

	// Handle context text change
	function handleContextChange(newContext: string) {
		contextText = newContext;
	}

	async function uploadToS3(presignedUrl: string, file: File) {
		return new Promise<void>((resolve, reject) => {
			const xhr = new XMLHttpRequest();

			xhr.upload.addEventListener('progress', (event) => {
				if (event.lengthComputable) {
					uploadProgress = (event.loaded / event.total) * 100;
				}
			});

			xhr.addEventListener('load', () => {
				if (xhr.status === 200) {
					resolve();
				} else {
					reject(new Error(`Upload failed with status: ${xhr.status}`));
				}
			});

			xhr.addEventListener('error', () => {
				reject(new Error('Upload failed due to network error'));
			});

			xhr.open('PUT', presignedUrl);
			xhr.setRequestHeader('Content-Type', file.type);
			xhr.send(file);
		});
	}

	async function handleUpload() {
		if (!selectedFile) {
			error = 'Please select a file to upload';
			return;
		}

		const includedHeaders = getIncludedHeaders();
		if (includedHeaders.length === 0) {
			error = 'Please include at least one column for upload';
			return;
		}

		if (!presignedUrl) {
			error = 'Presigned URL not available. Please check your environment configuration.';
			return;
		}

		uploading = true;
		uploadProgress = 0;
		uploadStatus = `Uploading ${selectedFile.name}...`;
		error = '';

		try {
			const typeSchema = getColumnTypeSchema();

			uploadStatus = `Uploading ${selectedFile.name} to S3...`;
			await uploadToS3(presignedUrl, selectedFile);

			uploadStatus = `Upload successful! Processing ${includedHeaders.length} columns.`;
			uploadProgress = 100;

			// Include context in the API call
			const response = await parseCsvToParquet(
				data.env.CORE_API_URL,
				typeSchema,
				key,
				job_id,
				contextText,
				columnTypes
			);

			if (response.statusCode !== 200) {
				console.log('failed');
			} else {
				console.log(response.parquet_key);
				await goto(`/chat?id=${job_id}`);
			}
		} catch (err) {
			error = err instanceof Error ? err.message : 'Upload failed';
			uploadStatus = '';
		} finally {
			uploading = false;
		}
	}

	function resetForm() {
		// Reset Upload component
		if (typeof window !== 'undefined' && (globalThis as any).__fileUploadReset) {
			(globalThis as any).__fileUploadReset();
		}
		// Reset local state
		handleReset();
	}

	function getIncludedHeaders(): string[] {
		return csvHeaders.filter((header) => !excludedColumns.has(header));
	}

	function getColumnTypeSchema() {
		return csvHeaders
			.filter((header) => !excludedColumns.has(header))
			.map((header) => ({
				column: header,
				type: columnTypes[header]
			}));
	}

	function handleColumnTypesChange(newTypes: { [key: string]: string }) {
		columnTypes = newTypes;
	}

	function handleExcludedColumnsChange(newExcluded: Set<string>) {
		excludedColumns = newExcluded;
	}
</script>

<div class="page-container">
	<h1>Buzz CSV</h1>
	<p class="subtitle">Upload your CSV files so that you can query your data in plain english</p>
	<div class="main-content">
		<div class="upload-container">
			<Upload
				disabled={uploading}
				onFileSelect={handleFileSelect}
				onHeadersRead={handleHeadersRead}
				onDataRead={handleDataRead}
				onError={handleUploadError}
				onReset={handleReset}
			/>

			<Colums
				headers={csvHeaders}
				data={csvData}
				bind:columnTypes
				bind:excludedColumns
				onColumnTypesChange={handleColumnTypesChange}
				onExcludedColumnsChange={handleExcludedColumnsChange}
			/>

			{#if csvHeaders.length > 0}
				<ContextBox {contextText} onContextChange={handleContextChange} disabled={uploading} />
			{/if}

			<div class="button-group">
				<button
					onclick={handleUpload}
					disabled={uploading || !selectedFile || getIncludedHeaders().length === 0}
				>
					{uploading ? 'Uploading...' : `Upload ${getIncludedHeaders().length} columns to S3`}
				</button>

				{#if uploadStatus || error || csvHeaders.length > 0}
					<button onclick={resetForm} disabled={uploading}> Reset </button>
				{/if}
			</div>
			<UploadingStatus {uploading} {uploadProgress} {uploadStatus} {error} />
		</div>
	</div>
</div>

<style>
	:global(body) {
		margin: 0;
		padding: 0;
		min-height: 100vh;
		background: linear-gradient(135deg, #232f3e 0%, #131a22 100%);
		font-family: 'Amazon Ember', 'Helvetica Neue', Roboto, Arial, sans-serif;
	}

	:global(*) {
		box-sizing: border-box;
	}

	.page-container {
		min-height: 100vh;
		display: flex;
		flex-direction: column;
		align-items: center;
		justify-content: center;
		padding: 2rem;
	}

	.main-content {
		width: 100%;
		max-width: 800px;
		text-align: center;
	}

	h1 {
		color: #ffffff;
		font-weight: 700;
		font-size: 2.5rem;
		margin-bottom: 0.5rem;
		text-shadow: 0 2px 4px rgba(0, 0, 0, 0.3);
		letter-spacing: -0.025em;
	}

	.subtitle {
		color: rgba(255, 255, 255, 0.8);
		font-size: 1.1rem;
		margin-bottom: 3rem;
		font-weight: 300;
	}

	.upload-container {
		background: #ffffff;
		border: 1px solid #d5dbdb;
		border-radius: 8px;
		padding: 2.5rem;
		box-shadow:
			0 4px 6px -1px rgba(0, 0, 0, 0.1),
			0 2px 4px -1px rgba(0, 0, 0, 0.06);
		text-align: left;
		width: 100%;
	}

	.button-group {
		display: flex;
		gap: 1rem;
		margin-bottom: 2rem;
	}

	button {
		padding: 0.75rem 1.5rem;
		background: #ff9900;
		color: #000000;
		border: 1px solid #ff9900;
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.95rem;
		font-weight: 700;
		font-family: inherit;
		transition: all 0.2s ease;
		position: relative;
		overflow: hidden;
	}

	button:hover:not(:disabled) {
		background: #e88b00;
		border-color: #e88b00;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
	}

	button:active:not(:disabled) {
		background: #cc7700;
		border-color: #cc7700;
	}

	button:disabled {
		background: #aab7b8;
		border-color: #aab7b8;
		color: #5a6c75;
		cursor: not-allowed;
		opacity: 0.6;
	}

	@keyframes shimmer {
		0% {
			transform: translateX(-100%);
		}
		100% {
			transform: translateX(100%);
		}
	}

	@media (max-width: 768px) {
		.page-container {
			padding: 1rem;
		}

		.upload-container {
			padding: 1.5rem;
		}

		h1 {
			font-size: 2rem;
		}

		.button-group {
			flex-direction: column;
		}

		button {
			width: 100%;
		}
	}
</style>
