<script lang="ts">
	interface Props {
		disabled?: boolean;
		onFileSelect?: (file: File) => void;
		onHeadersRead?: (headers: string[]) => void;
		onDataRead?: (data: any[][]) => void; // Add data callback
		onError?: (error: string) => void;
		onReset?: () => void;
	}

	let {
		disabled = false,
		onFileSelect,
		onHeadersRead,
		onDataRead, // Add data callback prop
		onError,
		onReset
	}: Props = $props();

	let files = $state<FileList | null>(null);
	let isReadingHeaders = $state(false);
	let isReadingData = $state(false); // Add data reading state
	let headerError = $state('');
	let fileInputElement: HTMLInputElement;

	// Computed values
	const selectedFile = $derived(files?.[0] || null);
	const isProcessing = $derived(isReadingHeaders || isReadingData);

	function validateCSVFile(file: File): boolean {
		const fileName = file.name.toLowerCase();
		if (!fileName.endsWith('.csv')) {
			return false;
		}

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

		result.push(current.trim());

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

			const firstLineEnd = text.indexOf('\n');
			const firstLine = firstLineEnd !== -1 ? text.substring(0, firstLineEnd) : text;

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

	async function readCSVData(file: File, maxRows: number = 1000): Promise<any[][]> {
		try {
			// For large files, read in chunks. For smaller files, read entirely.
			const maxFileSize = 5 * 1024 * 1024; // 5MB threshold
			const shouldSample = file.size > maxFileSize;

			let text: string;

			if (shouldSample) {
				// For large files, read first chunk for sampling
				const chunk = file.slice(0, maxFileSize);
				text = await chunk.text();
			} else {
				// For smaller files, read entire file
				text = await file.text();
			}

			const lines = text.split('\n').filter((line) => line.trim() !== '');

			if (lines.length <= 1) {
				return []; // No data rows (only headers or empty)
			}

			// Skip the header row and take up to maxRows for type inference
			const dataLines = lines.slice(1, Math.min(lines.length, maxRows + 1));
			const parsedData: any[][] = [];

			for (const line of dataLines) {
				try {
					const row = parseCSVRow(line.trim());
					if (row.length > 0) {
						// Convert values to appropriate types for better inference
						const convertedRow = row.map((value) => {
							if (value === '' || value === null || value === undefined) {
								return null;
							}

							// Try to convert to number if it looks like one
							const trimmed = value.trim();

							// More aggressive numeric detection
							if (/^-?\d*\.?\d+$/.test(trimmed)) {
								const num = Number(trimmed);
								if (!isNaN(num) && isFinite(num)) {
									return num;
								}
							}

							// Keep as string
							return trimmed;
						});
						parsedData.push(convertedRow);
					}
				} catch (rowError) {
					// Skip malformed rows
					console.warn('Skipped malformed row:', line);
				}
			}

			return parsedData;
		} catch (err) {
			throw new Error(
				`Failed to read CSV data: ${err instanceof Error ? err.message : 'Unknown error'}`
			);
		}
	}

	async function handleFileChange() {
		headerError = '';

		if (!files || files.length === 0) {
			onReset?.();
			return;
		}

		const file = files[0];

		if (!validateCSVFile(file)) {
			const error = 'Please select a valid CSV file (.csv extension required)';
			headerError = error;
			onError?.(error);
			return;
		}

		// Notify parent that a file was selected
		onFileSelect?.(file);

		try {
			// Step 1: Read headers
			isReadingHeaders = true;
			const headers = await readCSVHeaders(file);
			onHeadersRead?.(headers);

			// Step 2: Read data for type inference (if callback provided)
			if (onDataRead) {
				isReadingHeaders = false;
				isReadingData = true;

				const data = await readCSVData(file);
				onDataRead(data);
			}
		} catch (err) {
			const error = err instanceof Error ? err.message : 'Failed to read CSV file';
			headerError = error;
			onError?.(error);
		} finally {
			isReadingHeaders = false;
			isReadingData = false;
		}
	}

	function resetFileInput() {
		files = null;
		headerError = '';
		if (fileInputElement) {
			fileInputElement.value = '';
		}
		onReset?.();
	}

	// Expose reset function to parent
	function reset() {
		resetFileInput();
	}

	// Make reset function available to parent component
	$effect(() => {
		if (typeof window !== 'undefined') {
			// Store reference for parent access if needed
			(globalThis as any).__fileUploadReset = reset;
		}
	});
</script>

<div class="file-upload-container">
	<div class="form-group">
		<label for="file-input">Select CSV file to upload:</label>
		<input
			id="file-input"
			bind:this={fileInputElement}
			type="file"
			accept=".csv,text/csv"
			bind:files
			onchange={handleFileChange}
			disabled={disabled || isProcessing}
		/>
	</div>

	{#if selectedFile}
		<div class="file-info">
			<div class="file-details">
				<p><strong>Selected:</strong> {selectedFile.name}</p>
				<p><strong>Size:</strong> {(selectedFile.size / 1024 / 1024).toFixed(2)} MB</p>
				<p><strong>Type:</strong> {selectedFile.type || 'text/csv'}</p>
			</div>
			<button
				type="button"
				class="reset-btn"
				onclick={resetFileInput}
				disabled={disabled || isProcessing}
				title="Remove selected file"
			>
				✕
			</button>
		</div>
	{/if}

	{#if isReadingHeaders}
		<div class="status info">
			<div class="loading-spinner"></div>
			Reading CSV headers...
		</div>
	{/if}

	{#if isReadingData}
		<div class="status info">
			<div class="loading-spinner"></div>
			Analyzing data for automatic type detection...
		</div>
	{/if}

	{#if headerError}
		<div class="status error">
			<strong>Error:</strong>
			{headerError}
		</div>
	{/if}
</div>

<style>
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

	input[type='file']:hover:not(:disabled) {
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
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.file-details {
		flex: 1;
	}

	.file-info p {
		margin: 0.5rem 0;
		font-size: 0.95rem;
		opacity: 0.95;
	}

	.reset-btn {
		background: rgba(255, 255, 255, 0.1);
		border: 1px solid rgba(255, 255, 255, 0.2);
		color: white;
		padding: 0.5rem;
		border-radius: 50%;
		width: 32px;
		height: 32px;
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		font-size: 1rem;
		font-weight: bold;
		transition: all 0.2s ease;
		margin-left: 1rem;
		flex-shrink: 0;
	}

	.reset-btn:hover:not(:disabled) {
		background: rgba(255, 255, 255, 0.2);
		border-color: rgba(255, 255, 255, 0.4);
	}

	.reset-btn:disabled {
		opacity: 0.4;
		cursor: not-allowed;
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
		display: flex;
		align-items: center;
		gap: 0.75rem;
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

	.loading-spinner {
		width: 16px;
		height: 16px;
		border: 2px solid #b3d9ff;
		border-top: 2px solid #146eb4;
		border-radius: 50%;
		animation: spin 1s linear infinite;
		flex-shrink: 0;
	}

	@keyframes spin {
		0% {
			transform: rotate(0deg);
		}
		100% {
			transform: rotate(360deg);
		}
	}

	@media (max-width: 768px) {
		.file-info {
			flex-direction: column;
			align-items: flex-start;
			gap: 1rem;
		}

		.reset-btn {
			margin-left: 0;
			align-self: flex-end;
		}
	}
</style>
