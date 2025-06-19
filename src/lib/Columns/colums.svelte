<script lang="ts">
	import DataKey from '../DataKey/data-key.svelte';

	interface Props {
		headers: string[];
		columnTypes?: { [key: string]: string };
		excludedColumns?: Set<string>;
		onColumnTypesChange?: (types: { [key: string]: string }) => void;
		onExcludedColumnsChange?: (excluded: Set<string>) => void;
	}

	let {
		headers = [],
		columnTypes = $bindable({}),
		excludedColumns = $bindable(new Set<string>()),
		onColumnTypesChange,
		onExcludedColumnsChange
	}: Props = $props();

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

	function updateColumnType(header: string, type: string) {
		columnTypes = { ...columnTypes, [header]: type };
		onColumnTypesChange?.(columnTypes);
	}

	function toggleColumnExclusion(header: string) {
		const newExcluded = new Set(excludedColumns);
		if (newExcluded.has(header)) {
			newExcluded.delete(header);
			if (!columnTypes[header]) {
				columnTypes = { ...columnTypes, [header]: 'string' };
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
		headers.forEach((header) => {
			if (!newTypes[header]) {
				newTypes[header] = 'string';
			}
		});
		columnTypes = newTypes;
		onExcludedColumnsChange?.(excludedColumns);
		onColumnTypesChange?.(columnTypes);
	}

	// Initialize column types when headers change
	$effect(() => {
		if (headers.length > 0 && Object.keys(columnTypes).length === 0) {
			const types: { [key: string]: string } = {};
			headers.forEach((header) => {
				types[header] = 'string';
			});
			columnTypes = types;
			onColumnTypesChange?.(columnTypes);
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

		<div class="column-types-grid">
			{#each headers as header, index}
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

	.bulk-btn:disabled {
		opacity: 0.5;
		cursor: not-allowed;
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
