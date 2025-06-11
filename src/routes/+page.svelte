<script lang="ts">
	import type { LayoutData } from './$types';

	let { data }: { data: LayoutData } = $props();

	let files = $state<FileList | null>(null);
	let uploading = $state(false);
	let uploadProgress = $state(0);
	let uploadStatus = $state('');
	let error = $state('');
	let csvHeaders = $state<string[]>([]);
	let columnTypes = $state<{ [key: string]: string }>({});
	let excludedColumns = $state<Set<string>>(new Set());
	let isReadingHeaders = $state(false);
	let headerError = $state('');

	const presignedUrl = data.env.PRESIGNED_URL;
	const key = data.env.KEY;
	console.log(key);

	// Available data types for columns
	const dataTypes = [
		{ value: 'string', label: 'String (Text)' },
		{ value: 'integer', label: 'Integer (Whole Number)' },
		{ value: 'float', label: 'Float (Decimal Number)' },
		{ value: 'boolean', label: 'Boolean (True/False)' },
		{ value: 'date', label: 'Date' },
		{ value: 'datetime', label: 'DateTime' },
		{ value: 'timestamp', label: 'Timestamp' }
	];

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

	function initializeColumnTypes(headers: string[]) {
		const types: { [key: string]: string } = {};
		headers.forEach((header) => {
			types[header] = 'string'; // Default to string type
		});
		columnTypes = types;
		excludedColumns = new Set(); // Reset excluded columns when new headers are loaded
	}

	function updateColumnType(header: string, type: string) {
		columnTypes = { ...columnTypes, [header]: type };
	}

	function setAllColumnsType(type: string) {
		const newTypes: { [key: string]: string } = {};
		csvHeaders.forEach((header) => {
			if (!excludedColumns.has(header)) {
				newTypes[header] = type;
			}
		});
		columnTypes = { ...columnTypes, ...newTypes };
	}

	function toggleColumnExclusion(header: string) {
		const newExcluded = new Set(excludedColumns);
		if (newExcluded.has(header)) {
			newExcluded.delete(header);
			// When re-including a column, set it to default string type if it doesn't have a type
			if (!columnTypes[header]) {
				columnTypes = { ...columnTypes, [header]: 'string' };
			}
		} else {
			newExcluded.add(header);
		}
		excludedColumns = newExcluded;
	}

	function includeAllColumns() {
		excludedColumns = new Set();
		// Ensure all headers have a type
		const newTypes = { ...columnTypes };
		csvHeaders.forEach((header) => {
			if (!newTypes[header]) {
				newTypes[header] = 'string';
			}
		});
		columnTypes = newTypes;
	}

	function excludeAllColumns() {
		excludedColumns = new Set(csvHeaders);
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

	async function handleFileChange() {
		// Reset states
		csvHeaders = [];
		columnTypes = {};
		excludedColumns = new Set();
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
			initializeColumnTypes(csvHeaders);
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

		const includedHeaders = getIncludedHeaders();
		if (includedHeaders.length === 0) {
			error = 'Please include at least one column for upload';
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
			// Get the column type schema (only for included columns)
			const typeSchema = getColumnTypeSchema();
			console.log('Column Type Schema:', typeSchema);
			console.log('Excluded Columns:', Array.from(excludedColumns));

			// TODO: Send typeSchema along with S3 key to your lambda
			// const lambdaPayload = {
			//   s3Key: key,
			//   columnTypes: typeSchema,
			//   excludedColumns: Array.from(excludedColumns)
			// };

			// Upload directly to S3 using the presigned URL from environment
			uploadStatus = `Uploading ${file.name} to S3...`;
			await uploadToS3(presignedUrl, file);

			uploadStatus = `Upload successful! Processing ${includedHeaders.length} columns.`;
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
		columnTypes = {};
		excludedColumns = new Set();
		headerError = '';
		// Reset file input
		const fileInput = document.getElementById('file-input') as HTMLInputElement;
		if (fileInput) fileInput.value = '';
	}
</script>

<h1>S3 CSV Upload</h1>
<p class="subtitle">Upload your CSV files with instant header preview and column type selection</p>

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
					<div class="headers-title">
						<h3>
							CSV Headers & Column Types
							<span class="column-count">
								({getIncludedHeaders().length}/{csvHeaders.length} columns included)
							</span>
						</h3>
						<div class="bulk-actions">
							<div class="bulk-action-group">
								<label>Columns:</label>
								<button
									type="button"
									class="bulk-btn include-all"
									onclick={includeAllColumns}
									disabled={excludedColumns.size === 0}
								>
									Include All
								</button>
								<button
									type="button"
									class="bulk-btn exclude-all"
									onclick={excludeAllColumns}
									disabled={excludedColumns.size === csvHeaders.length}
								>
									Exclude All
								</button>
							</div>
							<div class="bulk-action-group">
								<label>Set included to:</label>
								<select onchange={(e) => setAllColumnsType(e.target.value)}>
									<option value="">Select type...</option>
									{#each dataTypes as type}
										<option value={type.value}>{type.label}</option>
									{/each}
								</select>
							</div>
						</div>
					</div>

					<!-- Data Types Guide -->
					<details class="data-types-guide">
						<summary>📋 Data Types Guide - How to Choose</summary>
						<div class="guide-content">
							<div class="guide-section">
								<h4>Basic Data Types</h4>
								<div class="type-examples">
									<div class="type-example">
										<strong>String (Text):</strong> Names, descriptions, addresses
									</div>
									<div class="type-example">
										<strong>Integer:</strong> Whole numbers like 1, 42, 100
									</div>
									<div class="type-example">
										<strong>Float:</strong> Decimal numbers like 3.14, 25.99, 0.5
									</div>
									<div class="type-example">
										<strong>Boolean:</strong> Yes/No, True/False, 1/0 values
									</div>
								</div>
							</div>

							<div class="guide-section">
								<h4>Date & Time Types</h4>
								<div class="date-types">
									<div class="date-type">
										<div class="date-header">
											<strong>📅 Date</strong>
											<span class="use-when">Use when: Just calendar dates</span>
										</div>
										<div class="date-examples">
											<span>Examples: 2024-03-15, March 15, 2024, 03/15/2024</span>
										</div>
										<div class="date-common">
											<span>Common in: Birthdays, deadlines, order dates</span>
										</div>
									</div>

									<div class="date-type">
										<div class="date-header">
											<strong>🕐 DateTime</strong>
											<span class="use-when">Use when: Date + specific time</span>
										</div>
										<div class="date-examples">
											<span>Examples: 2024-03-15 14:30:00, March 15, 2024 2:30 PM</span>
										</div>
										<div class="date-common">
											<span>Common in: Meetings, appointments, scheduled events</span>
										</div>
									</div>

									<div class="date-type">
										<div class="date-header">
											<strong>⏱️ Timestamp</strong>
											<span class="use-when">Use when: Precise timing needed</span>
										</div>
										<div class="date-examples">
											<span>Examples: 2024-03-15 14:30:25.123, 1710509425123</span>
										</div>
										<div class="date-common">
											<span>Common in: System logs, transactions, sensor data</span>
										</div>
									</div>
								</div>
							</div>

							<div class="guide-section">
								<h4>Quick Decision Helper</h4>
								<div class="decision-helper">
									<div class="decision-step">
										<strong>1.</strong> Does your data include time of day?
										<div class="decision-options">
											<span class="option-no">❌ No → Use <strong>Date</strong></span>
											<span class="option-yes">✅ Yes → Continue to step 2</span>
										</div>
									</div>
									<div class="decision-step">
										<strong>2.</strong> Do you need precise timing (seconds/milliseconds)?
										<div class="decision-options">
											<span class="option-no">❌ No → Use <strong>DateTime</strong></span>
											<span class="option-yes">✅ Yes → Use <strong>Timestamp</strong></span>
										</div>
									</div>
									<div class="decision-tip">
										💡 <strong>When in doubt:</strong> DateTime is usually the safest choice for most
										business data
									</div>
								</div>
							</div>
						</div>
					</details>

					<div class="column-types-grid">
						{#each csvHeaders as header, index}
							{@const isExcluded = excludedColumns.has(header)}
							<div class="column-type-item" class:excluded={isExcluded}>
								<div class="column-controls">
									<button
										type="button"
										class="toggle-column-btn"
										class:include={isExcluded}
										class:exclude={!isExcluded}
										onclick={() => toggleColumnExclusion(header)}
										title={isExcluded ? 'Include this column' : 'Exclude this column'}
									>
										{isExcluded ? '✓' : '✕'}
									</button>
								</div>
								<div class="column-info">
									<span class="column-index">Col {index + 1}:</span>
									<span class="column-name" class:excluded-text={isExcluded}>
										{header || '(empty)'}
									</span>
								</div>
								<div class="type-selector">
									<select
										bind:value={columnTypes[header]}
										onchange={(e) => updateColumnType(header, e.target.value)}
										disabled={isExcluded}
									>
										{#each dataTypes as type}
											<option value={type.value}>{type.label}</option>
										{/each}
									</select>
								</div>
							</div>
						{/each}
					</div>

					{#if excludedColumns.size > 0}
						<div class="excluded-summary">
							<strong>Excluded columns ({excludedColumns.size}):</strong>
							{Array.from(excludedColumns).join(', ')}
						</div>
					{/if}

					<details class="type-schema">
						<summary>Column Type Schema (for Lambda)</summary>
						<code>{JSON.stringify(getColumnTypeSchema(), null, 2)}</code>
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
				<button
					onclick={handleUpload}
					disabled={uploading || !files || getIncludedHeaders().length === 0}
				>
					{uploading ? 'Uploading...' : `Upload ${getIncludedHeaders().length} columns to S3`}
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

	.headers-title {
		display: flex;
		justify-content: space-between;
		align-items: flex-start;
		margin-bottom: 1.5rem;
		flex-wrap: wrap;
		gap: 1rem;
	}

	.headers-title h3 {
		margin: 0;
		color: #146eb4;
		font-size: 1.25rem;
		font-weight: 700;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.column-count {
		font-size: 0.9rem;
		color: #666;
		font-weight: 400;
	}

	.bulk-actions {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
		align-items: flex-end;
	}

	.bulk-action-group {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.9rem;
	}

	.bulk-action-group label {
		white-space: nowrap;
		color: #0f1419;
		font-weight: 600;
		font-size: 0.9rem;
		margin-bottom: 0;
	}

	.bulk-btn {
		padding: 0.4rem 0.8rem;
		border: 1px solid;
		border-radius: 4px;
		font-size: 0.8rem;
		font-weight: 600;
		cursor: pointer;
		transition: all 0.2s ease;
		font-family: inherit;
	}

	.bulk-btn.include-all {
		background: #d4edda;
		color: #155724;
		border-color: #c3e6cb;
	}

	.bulk-btn.include-all:hover:not(:disabled) {
		background: #c3e6cb;
		border-color: #b3d9cc;
	}

	.bulk-btn.exclude-all {
		background: #f8d7da;
		color: #721c24;
		border-color: #f5c6cb;
	}

	.bulk-btn.exclude-all:hover:not(:disabled) {
		background: #f5c6cb;
		border-color: #f1b0b7;
	}

	.bulk-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.bulk-action-group select {
		padding: 0.5rem 0.75rem;
		border: 1px solid #d5dbdb;
		border-radius: 4px;
		font-size: 0.9rem;
		background: white;
		color: #0f1419;
		font-family: inherit;
		transition: all 0.2s ease;
	}

	.bulk-action-group select:focus {
		outline: none;
		border-color: #ff9900;
		box-shadow: 0 0 0 2px rgba(255, 153, 0, 0.2);
	}

	.column-types-grid {
		display: grid;
		gap: 0.75rem;
		margin-bottom: 1.5rem;
	}

	.column-type-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 1rem;
		background: #ffffff;
		border: 1px solid #d5dbdb;
		border-radius: 4px;
		gap: 1rem;
		transition: all 0.2s ease;
	}

	.column-type-item:hover {
		border-color: #146eb4;
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
	}

	.column-type-item.excluded {
		background: #f8f9fa;
		border-color: #dee2e6;
		opacity: 0.7;
	}

	.column-type-item.excluded:hover {
		border-color: #adb5bd;
		box-shadow: 0 1px 2px rgba(0, 0, 0, 0.05);
	}

	.column-controls {
		display: flex;
		align-items: center;
	}

	.toggle-column-btn {
		width: 32px;
		height: 32px;
		border-radius: 50%;
		border: 2px solid;
		background: white;
		cursor: pointer;
		font-weight: bold;
		font-size: 0.9rem;
		transition: all 0.2s ease;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.toggle-column-btn.include {
		color: #28a745;
		border-color: #28a745;
	}

	.toggle-column-btn.include:hover {
		background: #28a745;
		color: white;
	}

	.toggle-column-btn.exclude {
		color: #dc3545;
		border-color: #dc3545;
	}

	.toggle-column-btn.exclude:hover {
		background: #dc3545;
		color: white;
	}

	.column-info {
		flex: 1;
		min-width: 0;
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.column-index {
		font-weight: 700;
		color: #146eb4;
		min-width: 70px;
		font-size: 0.9rem;
		background: #e6f3ff;
		padding: 0.25rem 0.5rem;
		border-radius: 4px;
		white-space: nowrap;
	}

	.column-name {
		color: #0f1419;
		font-family: 'Courier New', 'Monaco', 'Cascadia Code', 'Roboto Mono', monospace;
		background: #fafafa;
		padding: 0.5rem 0.75rem;
		border-radius: 4px;
		flex: 1;
		font-size: 0.9rem;
		border: 1px solid #d5dbdb;
		font-weight: 400;
		word-break: break-all;
		min-width: 0;
	}

	.column-name.excluded-text {
		text-decoration: line-through;
		color: #6c757d;
		background: #e9ecef;
	}

	.type-selector select {
		padding: 0.5rem 0.75rem;
		border: 1px solid #d5dbdb;
		border-radius: 4px;
		background: white;
		font-size: 0.9rem;
		min-width: 180px;
		color: #0f1419;
		font-family: inherit;
		transition: all 0.2s ease;
	}

	.type-selector select:focus {
		outline: none;
		border-color: #ff9900;
		box-shadow: 0 0 0 2px rgba(255, 153, 0, 0.2);
	}

	.type-selector select:disabled {
		background: #f8f9fa;
		color: #6c757d;
		cursor: not-allowed;
	}

	.excluded-summary {
		background: #fff3cd;
		border: 1px solid #ffeaa7;
		border-radius: 4px;
		padding: 1rem;
		margin-bottom: 1rem;
		font-size: 0.9rem;
		color: #856404;
	}

	.data-types-guide {
		margin-bottom: 2rem;
		border: 1px solid #d5dbdb;
		border-radius: 4px;
		background: #ffffff;
	}

	.data-types-guide summary {
		cursor: pointer;
		font-weight: 600;
		color: #146eb4;
		padding: 1rem;
		background: #f8f9fa;
		border-radius: 4px 4px 0 0;
		transition: all 0.2s ease;
		user-select: none;
		font-size: 1rem;
	}

	.data-types-guide summary:hover {
		background: #e9ecef;
		color: #0d5aa7;
	}

	.guide-content {
		padding: 1.5rem;
		border-top: 1px solid #d5dbdb;
	}

	.guide-section {
		margin-bottom: 2rem;
	}

	.guide-section:last-child {
		margin-bottom: 0;
	}

	.guide-section h4 {
		color: #146eb4;
		margin: 0 0 1rem 0;
		font-size: 1.1rem;
		font-weight: 700;
		border-bottom: 2px solid #e6f3ff;
		padding-bottom: 0.5rem;
	}

	.type-examples {
		display: grid;
		gap: 0.75rem;
	}

	.type-example {
		padding: 0.75rem;
		background: #f8f9fa;
		border-left: 3px solid #146eb4;
		border-radius: 0 4px 4px 0;
		font-size: 0.9rem;
	}

	.type-example strong {
		color: #146eb4;
	}

	.date-types {
		display: grid;
		gap: 1rem;
	}

	.date-type {
		border: 1px solid #e6f3ff;
		border-radius: 6px;
		padding: 1rem;
		background: #fafbfc;
	}

	.date-header {
		display: flex;
		justify-content: space-between;
		align-items: center;
		margin-bottom: 0.5rem;
		flex-wrap: wrap;
		gap: 0.5rem;
	}

	.date-header strong {
		color: #146eb4;
		font-size: 1rem;
	}

	.use-when {
		font-size: 0.85rem;
		color: #6c757d;
		font-style: italic;
		background: #e6f3ff;
		padding: 0.25rem 0.5rem;
		border-radius: 12px;
	}

	.date-examples {
		margin-bottom: 0.5rem;
		font-size: 0.9rem;
		color: #495057;
		font-family: 'Courier New', 'Monaco', 'Cascadia Code', 'Roboto Mono', monospace;
		background: #f1f3f4;
		padding: 0.5rem;
		border-radius: 4px;
	}

	.date-common {
		font-size: 0.85rem;
		color: #6c757d;
	}

	.decision-helper {
		background: #fff3cd;
		border: 1px solid #ffeaa7;
		border-radius: 6px;
		padding: 1rem;
	}

	.decision-step {
		margin-bottom: 1rem;
		font-size: 0.95rem;
	}

	.decision-step:last-of-type {
		margin-bottom: 0.75rem;
	}

	.decision-options {
		margin-top: 0.5rem;
		margin-left: 1.5rem;
		display: flex;
		flex-direction: column;
		gap: 0.25rem;
	}

	.option-no {
		color: #dc3545;
		font-size: 0.9rem;
	}

	.option-yes {
		color: #28a745;
		font-size: 0.9rem;
	}

	.decision-tip {
		background: #e8f4fd;
		border: 1px solid #bee5eb;
		border-radius: 4px;
		padding: 0.75rem;
		margin-top: 1rem;
		font-size: 0.9rem;
		color: #0c5460;
	}

	.decision-tip strong {
		color: #146eb4;
	}

	.type-schema {
		margin-top: 1rem;
	}

	.type-schema summary {
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

	.type-schema summary:hover {
		background: #f2f3f3;
		border-color: #146eb4;
	}

	.type-schema code {
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
		word-break: break-word;
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

		.headers-title {
			flex-direction: column;
			align-items: stretch;
		}

		.bulk-actions {
			align-items: stretch;
		}

		.bulk-action-group {
			justify-content: space-between;
		}

		.column-type-item {
			flex-direction: column;
			align-items: stretch;
			gap: 0.75rem;
		}

		.column-info {
			flex-direction: column;
			align-items: stretch;
			gap: 0.5rem;
		}

		.type-selector select {
			min-width: auto;
			width: 100%;
		}

		.toggle-column-btn {
			align-self: flex-start;
		}
	}
</style>
