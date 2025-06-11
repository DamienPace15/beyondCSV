<script lang="ts">
	import type { LayoutData } from './$types';

	let { data }: { data: LayoutData } = $props();

	let files = $state<FileList | null>(null);
	let uploading = $state(false);
	let uploadProgress = $state(0);
	let uploadStatus = $state('');
	let error = $state('');
	let csvHeaders = $state<string[]>([]);
	let isReadingHeaders = $state(false);
	let headerError = $state('');

	const presignedUrl = data.env.PRESIGNED_URL;
	const key = data.env.KEY;
	console.log(key);

	function validateCSVFile(file: File): boolean {
		// Check file extension
		const fileName = file.name.toLowerCase();
		if (!fileName.endsWith('.csv')) {
			return false;
		}

		// Check MIME type (optional, as CSV can have different MIME types)
		const validMimeTypes = [
			'text/csv',
			'application/csv',
			'text/plain',
			'application/vnd.ms-excel'
		];

		return validMimeTypes.includes(file.type) || file.type === '';
	}

	function parseCSVRow(row: string): string[] {
		const result: string[] = [];
		let current = '';
		let inQuotes = false;

		for (let i = 0; i < row.length; i++) {
			const char = row[i];

			if (char === '"') {
				inQuotes = !inQuotes;
			} else if (char === ',' && !inQuotes) {
				result.push(current.trim());
				current = '';
			} else {
				current += char;
			}
		}

		// Add the last field
		result.push(current.trim());

		// Remove quotes from fields that are fully quoted
		return result.map((field) => {
			if (field.startsWith('"') && field.endsWith('"')) {
				return field.slice(1, -1);
			}
			return field;
		});
	}

	async function readCSVHeaders(file: File): Promise<string[]> {
		try {
			// Read only the first 1KB of the file (should be enough for headers)
			const chunk = file.slice(0, 1024);
			const text = await chunk.text();

			// Find the first line break to get just the first row
			const firstLineEnd = text.indexOf('\n');
			const firstLine = firstLineEnd !== -1 ? text.substring(0, firstLineEnd) : text;

			// Parse the first row
			const headers = parseCSVRow(firstLine.trim());

			if (headers.length === 0) {
				throw new Error('CSV file appears to be empty or has no headers');
			}

			return headers;
		} catch (err) {
			throw new Error(
				`Failed to read CSV headers: ${err instanceof Error ? err.message : 'Unknown error'}`
			);
		}
	}

	async function handleFileChange() {
		// Reset states
		csvHeaders = [];
		headerError = '';
		error = '';
		uploadStatus = '';

		if (!files || files.length === 0) {
			return;
		}

		const file = files[0];

		// Validate CSV file
		if (!validateCSVFile(file)) {
			error = 'Please select a valid CSV file (.csv extension required)';
			return;
		}

		// Read CSV headers
		isReadingHeaders = true;
		try {
			csvHeaders = await readCSVHeaders(file);
		} catch (err) {
			headerError = err instanceof Error ? err.message : 'Failed to read headers';
		} finally {
			isReadingHeaders = false;
		}
	}

	async function uploadToS3(presignedUrl: string, file: File) {
		return new Promise<void>((resolve, reject) => {
			const xhr = new XMLHttpRequest();

			// Track upload progress
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
		if (!files || files.length === 0) {
			error = 'Please select a file to upload';
			return;
		}

		const file = files[0];

		// Validate CSV file
		if (!validateCSVFile(file)) {
			error = 'Please select a valid CSV file (.csv extension required)';
			return;
		}

		if (!presignedUrl) {
			error = 'Presigned URL not available. Please check your environment configuration.';
			return;
		}

		uploading = true;
		uploadProgress = 0;
		uploadStatus = `Uploading ${file.name}...`;
		error = '';

		try {
			// Upload directly to S3 using the presigned URL from environment
			uploadStatus = `Uploading ${file.name} to S3...`;
			await uploadToS3(presignedUrl, file);

			uploadStatus = `Upload successful!`;
			uploadProgress = 100;
		} catch (err) {
			error = err instanceof Error ? err.message : 'Upload failed';
			uploadStatus = '';
		} finally {
			uploading = false;
		}
	}

	function resetForm() {
		files = null;
		uploadProgress = 0;
		uploadStatus = '';
		error = '';
		csvHeaders = [];
		headerError = '';
		// Reset file input
		const fileInput = document.getElementById('file-input') as HTMLInputElement;
		if (fileInput) fileInput.value = '';
	}
</script>

<h1>S3 CSV Upload</h1>
<p class="subtitle">Upload your CSV files with instant header preview</p>

<div class="page-container">
	<div class="main-content">
		<div class="upload-container">
			<div class="form-group">
				<label for="file-input">Select CSV file to upload:</label>
				<input
					id="file-input"
					type="file"
					accept=".csv,text/csv"
					bind:files
					onchange={handleFileChange}
					disabled={uploading}
				/>
			</div>

			{#if files && files.length > 0}
				<div class="file-info">
					<p><strong>Selected:</strong> {files[0].name}</p>
					<p><strong>Size:</strong> {(files[0].size / 1024 / 1024).toFixed(2)} MB</p>
					<p><strong>Type:</strong> {files[0].type || 'text/csv'}</p>
				</div>
			{/if}

			{#if isReadingHeaders}
				<div class="status info">Reading CSV headers...</div>
			{/if}

			{#if csvHeaders.length > 0}
				<div class="headers-section">
					<h3>CSV Headers ({csvHeaders.length} columns)</h3>
					<div class="headers-grid">
						{#each csvHeaders as header, index}
							<div class="header-item">
								<span class="header-index">Col {index + 1}:</span>
								<span class="header-value">{header || '(empty)'}</span>
							</div>
						{/each}
					</div>
					<details class="raw-headers">
						<summary>Raw Array Output</summary>
						<code>{JSON.stringify(csvHeaders, null, 2)}</code>
					</details>
				</div>
			{/if}

			{#if headerError}
				<div class="status error">
					<strong>Header Error:</strong>
					{headerError}
				</div>
			{/if}

			<div class="button-group">
				<button onclick={handleUpload} disabled={uploading || !files || csvHeaders.length === 0}>
					{uploading ? 'Uploading...' : 'Upload to S3'}
				</button>

				{#if uploadStatus || error || csvHeaders.length > 0}
					<button onclick={resetForm} disabled={uploading}> Reset </button>
				{/if}
			</div>

			{#if uploading && uploadProgress > 0}
				<div class="progress-container">
					<div class="progress-bar">
						<div class="progress-fill" style="width: {uploadProgress}%"></div>
					</div>
					<span class="progress-text">{uploadProgress.toFixed(0)}%</span>
				</div>
			{/if}

			{#if uploadStatus}
				<div class="status success">
					{uploadStatus}
				</div>
			{/if}

			{#if error}
				<div class="status error">
					{error}
				</div>
			{/if}
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

	.form-group {
		margin-bottom: 2rem;
	}

	label {
		display: block;
		margin-bottom: 1rem;
		font-weight: 700;
		color: #0f1419;
		font-size: 1rem;
	}

	input[type='file'] {
		width: 100%;
		padding: 1rem;
		border: 2px dashed #aab7b8;
		border-radius: 4px;
		background: #fafafa;
		color: #0f1419;
		font-family: inherit;
		font-size: 0.95rem;
		transition: all 0.2s ease;
		cursor: pointer;
	}

	input[type='file']:hover {
		border-color: #ff9900;
		background: #fff3cd;
	}

	input[type='file']:focus {
		outline: none;
		border-color: #ff9900;
		background: #fff3cd;
		box-shadow: 0 0 0 2px rgba(255, 153, 0, 0.2);
	}

	input[type='file']:disabled {
		background: #f3f4f6;
		border-color: #d1d5db;
		cursor: not-allowed;
		opacity: 0.6;
	}

	.file-info {
		background: linear-gradient(90deg, #232f3e 0%, #37475a 100%);
		color: white;
		padding: 1.5rem;
		border-radius: 4px;
		margin-bottom: 2rem;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
		border-left: 4px solid #ff9900;
	}

	.file-info p {
		margin: 0.5rem 0;
		font-size: 0.95rem;
		opacity: 0.95;
	}

	.headers-section {
		background: #f2f3f3;
		padding: 2rem;
		border-radius: 4px;
		margin-bottom: 2rem;
		border: 1px solid #d5dbdb;
		border-left: 4px solid #146eb4;
	}

	.headers-section h3 {
		margin: 0 0 1.5rem 0;
		color: #146eb4;
		font-size: 1.25rem;
		font-weight: 700;
		display: flex;
		align-items: center;
		gap: 0.5rem;
	}

	.headers-grid {
		display: grid;
		gap: 0.75rem;
		margin-bottom: 1.5rem;
	}

	.header-item {
		display: flex;
		align-items: center;
		padding: 1rem;
		background: #ffffff;
		border-radius: 4px;
		border: 1px solid #d5dbdb;
		transition: all 0.2s ease;
	}

	.header-item:hover {
		border-color: #146eb4;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
	}

	.header-index {
		font-weight: 700;
		color: #146eb4;
		min-width: 70px;
		font-size: 0.9rem;
		background: #e6f3ff;
		padding: 0.25rem 0.5rem;
		border-radius: 4px;
		margin-right: 1rem;
	}

	.header-value {
		color: #0f1419;
		font-family: 'Courier New', 'Monaco', 'Cascadia Code', 'Roboto Mono', monospace;
		background: #fafafa;
		padding: 0.5rem 0.75rem;
		border-radius: 4px;
		flex: 1;
		font-size: 0.9rem;
		border: 1px solid #d5dbdb;
		font-weight: 400;
	}

	.raw-headers {
		margin-top: 1.5rem;
	}

	.raw-headers summary {
		cursor: pointer;
		font-weight: 600;
		color: #146eb4;
		padding: 1rem;
		background: #ffffff;
		border-radius: 4px;
		border: 1px solid #d5dbdb;
		transition: all 0.2s ease;
		user-select: none;
	}

	.raw-headers summary:hover {
		background: #f2f3f3;
		border-color: #146eb4;
	}

	.raw-headers code {
		display: block;
		background: #232f3e;
		color: #ffffff;
		padding: 1.5rem;
		border-radius: 4px;
		font-family: 'Courier New', 'Monaco', 'Cascadia Code', 'Roboto Mono', monospace;
		font-size: 0.85rem;
		white-space: pre-wrap;
		margin-top: 0.75rem;
		overflow-x: auto;
		border: 1px solid #37475a;
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

	.progress-container {
		display: flex;
		align-items: center;
		gap: 1.5rem;
		margin-bottom: 2rem;
	}

	.progress-bar {
		flex: 1;
		height: 8px;
		background: #d5dbdb;
		border-radius: 4px;
		overflow: hidden;
		border: 1px solid #aab7b8;
	}

	.progress-fill {
		height: 100%;
		background: linear-gradient(90deg, #ff9900 0%, #ffb84d 50%, #ff9900 100%);
		transition: width 0.3s ease;
		border-radius: 2px;
		position: relative;
	}

	.progress-fill::after {
		content: '';
		position: absolute;
		top: 0;
		left: 0;
		bottom: 0;
		right: 0;
		background: linear-gradient(
			90deg,
			transparent 30%,
			rgba(255, 255, 255, 0.3) 50%,
			transparent 70%
		);
		animation: shimmer 2s infinite;
	}

	@keyframes shimmer {
		0% {
			transform: translateX(-100%);
		}
		100% {
			transform: translateX(100%);
		}
	}

	.progress-text {
		font-weight: 700;
		min-width: 50px;
		color: #ff9900;
		font-size: 1rem;
	}

	.status {
		padding: 1rem 1.25rem;
		border-radius: 4px;
		margin-bottom: 1.5rem;
		font-weight: 500;
		font-size: 0.95rem;
		border: 1px solid;
		position: relative;
		overflow: hidden;
	}

	.status::before {
		content: '';
		position: absolute;
		top: 0;
		left: 0;
		right: 0;
		height: 4px;
		opacity: 1;
	}

	.status.success {
		background: #d4edda;
		color: #155724;
		border-color: #c3e6cb;
	}

	.status.success::before {
		background: #28a745;
	}

	.status.error {
		background: #f8d7da;
		color: #721c24;
		border-color: #f5c6cb;
	}

	.status.error::before {
		background: #dc3545;
	}

	.status.info {
		background: #cce7ff;
		color: #0c5460;
		border-color: #b3d9ff;
	}

	.status.info::before {
		background: #146eb4;
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
