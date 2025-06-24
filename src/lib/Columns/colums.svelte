<script lang="ts">
	import DataKey from '../DataKey/data-key.svelte';

	interface Props {
		headers: string[];
		data?: any[][]; // Add data prop for type inference
		columnTypes?: { [key: string]: string };
		excludedColumns?: Set<string>;
		onColumnTypesChange?: (types: { [key: string]: string }) => void;
		onExcludedColumnsChange?: (excluded: Set<string>) => void;
	}

	let {
		headers = [],
		data = [],
		columnTypes = $bindable({}),
		excludedColumns = $bindable(new Set<string>()),
		onColumnTypesChange,
		onExcludedColumnsChange
	}: Props = $props();

	// Track which types users have manually overridden
	let userModifiedTypes = $state(new Set<string>());

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

	// Computed values
	const includedHeaders = $derived(headers.filter((header) => !excludedColumns.has(header)));
	const excludedCount = $derived(excludedColumns.size);
	const hasUserOverrides = $derived(userModifiedTypes.size > 0);

	function inferColumnType(columnData) {
		if (!columnData || columnData.length === 0) return 'string';

		// Filter out null, undefined, and empty string values
		const validValues = columnData.filter((val) => val !== null && val !== undefined && val !== '');

		if (validValues.length === 0) return 'string';

		// Sample up to 100 values for performance
		const sampleSize = Math.min(validValues.length, 100);
		const sample = validValues.slice(0, sampleSize);

		let integerCount = 0;
		let floatCount = 0;
		let booleanCount = 0;
		let dateCount = 0;
		let datetimeCount = 0;

		for (const value of sample) {
			const strValue = String(value).trim();

			// Check for float FIRST (highest priority for numbers)
			if (isFloat(strValue)) {
				floatCount++;
				continue;
			}

			// Check for integer SECOND (only if not a float)
			if (isInteger(strValue)) {
				integerCount++;
				continue;
			}

			// Check for boolean
			if (isBooleanValue(strValue)) {
				booleanCount++;
				continue;
			}

			// Check for datetime before date (more specific pattern)
			if (isDateTime(strValue)) {
				datetimeCount++;
				continue;
			}

			// Check for date
			if (isDate(strValue)) {
				dateCount++;
				continue;
			}
		}

		const total = sample.length;
		const threshold = 0.8; // 80% of values should match the type

		// PRIORITY ORDER: FLOAT FIRST (highest priority), then INTEGER
		if (floatCount / total >= threshold) return 'float';
		if (integerCount / total >= threshold) return 'integer';
		if (booleanCount / total >= threshold) return 'boolean';
		if (datetimeCount / total >= threshold) return 'datetime';
		if (dateCount / total >= threshold) return 'date';

		// If mixed numeric types, ALWAYS prefer float over integer
		if ((integerCount + floatCount) / total >= threshold) {
			return floatCount > 0 ? 'float' : 'integer';
		}

		return 'string';
	}

	function isBooleanValue(value: string): boolean {
		const lower = value.toLowerCase();
		return ['true', 'false', '1', '0', 'yes', 'no', 'y', 'n'].includes(lower);
	}

	function isFloat(value) {
		const strValue = String(value).trim();
		const num = parseFloat(strValue);

		// Must be a valid number, be finite, and contain a decimal point in the original string
		return !isNaN(num) && isFinite(num) && strValue.includes('.');
	}

	function isInteger(value) {
		const strValue = String(value).trim();
		const num = Number(strValue);

		// Must be a valid number, be an integer, AND not contain a decimal point in the original string
		return !isNaN(num) && Number.isInteger(num) && isFinite(num) && !strValue.includes('.');
	}

	function isDate(value: string): boolean {
		// Common date patterns
		const datePatterns = [
			/^\d{4}-\d{2}-\d{2}$/, // YYYY-MM-DD
			/^\d{2}\/\d{2}\/\d{4}$/, // MM/DD/YYYY
			/^\d{2}-\d{2}-\d{4}$/, // MM-DD-YYYY
			/^\d{1,2}\/\d{1,2}\/\d{2,4}$/ // M/D/YY or MM/DD/YYYY
		];

		if (datePatterns.some((pattern) => pattern.test(value))) {
			const date = new Date(value);
			return !isNaN(date.getTime()) && date.getFullYear() > 1900 && date.getFullYear() < 2100;
		}

		return false;
	}

	function isDateTime(value: string): boolean {
		// Check for datetime patterns (date + time)
		const datetimePatterns = [
			/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/, // ISO format
			/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}/, // YYYY-MM-DD HH:MM:SS
			/^\d{2}\/\d{2}\/\d{4} \d{1,2}:\d{2}/ // MM/DD/YYYY H:MM
		];

		if (datetimePatterns.some((pattern) => pattern.test(value))) {
			const date = new Date(value);
			return !isNaN(date.getTime());
		}

		return false;
	}

	function getColumnData(headerIndex: number): any[] {
		if (!data || data.length === 0) return [];
		return data.map((row) => row[headerIndex]).filter((val) => val !== undefined);
	}

	function inferAllColumnTypes(): { [key: string]: string } {
		const inferredTypes: { [key: string]: string } = {};

		headers.forEach((header, index) => {
			const columnData = getColumnData(index);
			inferredTypes[header] = inferColumnType(columnData);
		});

		return inferredTypes;
	}

	function updateColumnType(header: string, type: string) {
		// Mark this column as user-modified
		userModifiedTypes.add(header);
		columnTypes = { ...columnTypes, [header]: type };
		onColumnTypesChange?.(columnTypes);
	}

	function toggleColumnExclusion(header: string) {
		const newExcluded = new Set(excludedColumns);
		if (newExcluded.has(header)) {
			newExcluded.delete(header);
			if (!columnTypes[header] || !userModifiedTypes.has(header)) {
				// Re-infer type when including column (only if not user-modified)
				const headerIndex = headers.indexOf(header);
				const columnData = getColumnData(headerIndex);
				const inferredType = inferColumnType(columnData);
				columnTypes = { ...columnTypes, [header]: inferredType };
			}
		} else {
			newExcluded.add(header);
		}
		excludedColumns = newExcluded;
		onExcludedColumnsChange?.(excludedColumns);
	}

	function includeAllColumns() {
		excludedColumns = new Set();
		const newTypes = { ...columnTypes };
		headers.forEach((header, index) => {
			if (!newTypes[header] || !userModifiedTypes.has(header)) {
				const columnData = getColumnData(index);
				newTypes[header] = inferColumnType(columnData);
			}
		});
		columnTypes = newTypes;
		onExcludedColumnsChange?.(excludedColumns);
		onColumnTypesChange?.(columnTypes);
	}

	function resetAutoDetection() {
		// Clear user modifications and re-infer all types
		userModifiedTypes.clear();
		const inferredTypes = inferAllColumnTypes();
		columnTypes = { ...inferredTypes };
		onColumnTypesChange?.(columnTypes);
	}

	// Automatic type inference when headers or data change
	$effect(() => {
		if (headers.length > 0 && data.length > 0) {
			const inferredTypes = inferAllColumnTypes();
			const newTypes = { ...columnTypes };
			let hasChanges = false;

			// Only auto-update types that user hasn't manually changed
			headers.forEach((header) => {
				if (!userModifiedTypes.has(header)) {
					if (!newTypes[header] || newTypes[header] !== inferredTypes[header]) {
						newTypes[header] = inferredTypes[header];
						hasChanges = true;
					}
				}
			});

			if (hasChanges) {
				columnTypes = newTypes;
				onColumnTypesChange?.(columnTypes);
			}
		}
	});
</script>

{#if headers.length > 0}
	<div class="headers-section">
		<div class="headers-title">
			<h3>
				CSV Headers & Column Types
				<span class="column-count">
					({includedHeaders.length}/{headers.length} columns included)
				</span>
			</h3>
			<div class="bulk-actions">
				{#if hasUserOverrides}
					<div class="bulk-action-group">
						<label>Auto-detection:</label>
						<button
							type="button"
							class="bulk-btn reinfer"
							onclick={resetAutoDetection}
							title="Clear all manual overrides and automatically re-detect all column types"
						>
							Reset Auto-Detection
						</button>
					</div>
				{/if}
				<div class="bulk-action-group">
					<label>Columns:</label>
					<button
						type="button"
						class="bulk-btn include-all"
						onclick={includeAllColumns}
						disabled={excludedCount === 0}
					>
						Include All
					</button>
				</div>
			</div>
		</div>

		<DataKey />

		<div class="inference-info">
			<p>
				<strong>Automatic type detection:</strong> Column types are automatically detected from your
				data as you work.
				{#if hasUserOverrides}
					Manual overrides are preserved - use "Reset Auto-Detection" to clear them.
				{:else}
					Types will update automatically when your data changes.
				{/if}
			</p>
		</div>

		<div class="column-types-grid">
			{#each headers as header, index}
				{@const isExcluded = excludedColumns.has(header)}
				{@const isUserModified = userModifiedTypes.has(header)}
				{@const isAutoDetected = !isUserModified && columnTypes[header]}
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
						{#if isAutoDetected && !isExcluded}
							<span class="inferred-badge auto" title="Type automatically detected from data">
								Auto
							</span>
						{:else if isUserModified && !isExcluded}
							<span class="inferred-badge manual" title="Type manually set by user"> Manual </span>
						{/if}
					</div>
					<div class="type-selector">
						<select
							bind:value={columnTypes[header]}
							onchange={(e) => updateColumnType(header, e.currentTarget.value)}
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

		{#if excludedCount > 0}
			<div class="excluded-summary">
				<strong>Excluded columns ({excludedCount}):</strong>
				{Array.from(excludedColumns).join(', ')}
			</div>
		{/if}
	</div>
{/if}

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

	label {
		display: block;
		margin-bottom: 1rem;
		font-weight: 700;
		color: #0f1419;
		font-size: 1rem;
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
		min-width: 200px;
	}

	.bulk-action-group {
		display: flex;
		align-items: center;
		gap: 0.5rem;
		font-size: 0.9rem;
		width: 100%;
		justify-content: flex-end;
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
		white-space: nowrap;
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

	.bulk-btn.reinfer {
		background: #fff3cd;
		color: #856404;
		border-color: #ffeaa7;
	}

	.bulk-btn.reinfer:hover {
		background: #ffeaa7;
		border-color: #ffdf7e;
	}

	.bulk-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
	}

	.inference-info {
		background: #e1ecf4;
		border: 1px solid #146eb4;
		border-radius: 4px;
		padding: 1rem;
		margin-bottom: 1.5rem;
		font-size: 0.9rem;
		color: #0c5aa6;
	}

	.inference-info p {
		margin: 0;
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

	.inferred-badge {
		color: white;
		font-size: 0.75rem;
		font-weight: 600;
		padding: 0.25rem 0.5rem;
		border-radius: 12px;
		white-space: nowrap;
		margin-left: auto;
	}

	.inferred-badge.auto {
		background: #28a745;
	}

	.inferred-badge.manual {
		background: #146eb4;
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

	@media (max-width: 768px) {
		button {
			width: 100%;
		}

		.headers-title {
			flex-direction: column;
			align-items: stretch;
		}

		.bulk-actions {
			align-items: stretch;
			min-width: auto;
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

		.inferred-badge {
			margin-left: 0;
			align-self: flex-start;
		}
	}
</style>
