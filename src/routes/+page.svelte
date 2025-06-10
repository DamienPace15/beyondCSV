<script lang="ts">
	import type { LayoutData } from './$types';

	let { data }: { data: LayoutData } = $props();

	let files = $state<FileList | null>(null);
	let uploading = $state(false);
	let uploadProgress = $state(0);
	let uploadStatus = $state('');
	let error = $state('');

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
		// Reset file input
		const fileInput = document.getElementById('file-input') as HTMLInputElement;
		if (fileInput) fileInput.value = '';
	}
</script>

<h1>S3 CSV Upload</h1>

<div class="upload-container">
	<div class="form-group">
		<label for="file-input">Select CSV file to upload:</label>
		<input id="file-input" type="file" accept=".csv,text/csv" bind:files disabled={uploading} />
	</div>

	{#if files && files.length > 0}
		<div class="file-info">
			<p><strong>Selected:</strong> {files[0].name}</p>
			<p><strong>Size:</strong> {(files[0].size / 1024 / 1024).toFixed(2)} MB</p>
			<p><strong>Type:</strong> {files[0].type || 'text/csv'}</p>
		</div>
	{/if}

	<div class="button-group">
		<button onclick={handleUpload} disabled={uploading || !files}>
			{uploading ? 'Uploading...' : 'Upload to S3'}
		</button>

		{#if uploadStatus || error}
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

<style>
	h1 {
		color: #232f3e;
		font-family: 'Amazon Ember', Arial, sans-serif;
		font-weight: 700;
		margin-bottom: 1.5rem;
	}

	.upload-container {
		max-width: 500px;
		margin: 2rem 0;
		padding: 1.5rem;
		border: 2px solid #d5dbdb;
		border-radius: 8px;
		background: #ffffff;
		box-shadow: 0 2px 4px rgba(35, 47, 62, 0.1);
		font-family: 'Amazon Ember', Arial, sans-serif;
	}

	.form-group {
		margin-bottom: 1.25rem;
	}

	label {
		display: block;
		margin-bottom: 0.5rem;
		font-weight: 600;
		color: #232f3e;
		font-size: 0.95rem;
	}

	input[type='file'] {
		width: 100%;
		padding: 0.75rem;
		border: 2px solid #d5dbdb;
		border-radius: 4px;
		background: #ffffff;
		color: #232f3e;
		font-family: inherit;
		transition: border-color 0.2s ease;
	}

	input[type='file']:focus {
		outline: none;
		border-color: #ff9900;
		box-shadow: 0 0 0 2px rgba(255, 153, 0, 0.2);
	}

	input[type='file']:disabled {
		background: #f2f3f3;
		border-color: #d5dbdb;
		cursor: not-allowed;
	}

	.file-info {
		background: #f2f3f3;
		padding: 1.25rem;
		border-radius: 6px;
		margin-bottom: 1.25rem;
		border-left: 4px solid #ff9900;
	}

	.file-info p {
		margin: 0.25rem 0;
		color: #232f3e;
		font-size: 0.9rem;
	}

	.button-group {
		display: flex;
		gap: 0.75rem;
		margin-bottom: 1.25rem;
	}

	button {
		padding: 0.75rem 1.5rem;
		background: #ff9900;
		color: #ffffff;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.95rem;
		font-weight: 600;
		font-family: inherit;
		transition: all 0.2s ease;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
	}

	button:hover:not(:disabled) {
		background: #e88b00;
		box-shadow: 0 4px 8px rgba(0, 0, 0, 0.15);
		transform: translateY(-1px);
	}

	button:active:not(:disabled) {
		transform: translateY(0);
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
	}

	button:disabled {
		background: #aab7b8;
		cursor: not-allowed;
		box-shadow: none;
		transform: none;
	}

	.progress-container {
		display: flex;
		align-items: center;
		gap: 1rem;
		margin-bottom: 1.25rem;
	}

	.progress-bar {
		flex: 1;
		height: 24px;
		background: #eaeded;
		border-radius: 12px;
		overflow: hidden;
		border: 1px solid #d5dbdb;
	}

	.progress-fill {
		height: 100%;
		background: linear-gradient(90deg, #ff9900 0%, #ffb84d 100%);
		transition: width 0.3s ease;
	}

	.progress-text {
		font-weight: 600;
		min-width: 40px;
		color: #232f3e;
		font-size: 0.9rem;
	}

	.status {
		padding: 1rem 1.25rem;
		border-radius: 6px;
		margin-top: 1rem;
		font-weight: 500;
		border-left-width: 4px;
		border-left-style: solid;
	}

	.status.success {
		background: #d5f4e6;
		color: #037f3c;
		border-left-color: #17b978;
		border: 1px solid #17b978;
		border-left-width: 4px;
	}

	.status.error {
		background: #fbeae5;
		color: #d13212;
		border-left-color: #d13212;
		border: 1px solid #d13212;
		border-left-width: 4px;
	}
</style>
