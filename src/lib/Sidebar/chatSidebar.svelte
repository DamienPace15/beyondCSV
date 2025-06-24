<script lang="ts">
	import type { ComponentProps } from 'svelte';

	interface Schema {
		[key: string]: string;
	}

	interface Props {
		visible: boolean;
		schema: Schema;
		context: string;
		isEditingContext: boolean;
		editableContext: string;
		onToggle: () => void;
		onStartEditingContext: () => void;
		onSaveContext: () => Promise<void>;
		onCancelEditContext: () => void;
		onContextChange: (value: string) => void;
	}

	let {
		visible = false,
		schema = {},
		context = '',
		isEditingContext = false,
		editableContext = '',
		onToggle,
		onStartEditingContext,
		onSaveContext,
		onCancelEditContext,
		onContextChange
	}: Props = $props();

	function handleContextInput(event: Event) {
		const target = event.target as HTMLTextAreaElement;
		onContextChange(target.value);
	}
</script>

<!-- Sidebar Toggle Button -->
<button class="sidebar-toggle" onclick={onToggle} aria-label="Toggle sidebar">
	{#if visible}
		<svg
			width="20"
			height="20"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
		>
			<path d="M21 3H3c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h18c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2z" />
			<path d="M10 4v16" />
			<path d="m14 8 2 2-2 2" />
		</svg>
	{:else}
		<svg
			width="20"
			height="20"
			viewBox="0 0 24 24"
			fill="none"
			stroke="currentColor"
			stroke-width="2"
		>
			<path d="M21 3H3c-1.1 0-2 .9-2 2v14c0 1.1.9 2 2 2h18c1.1 0 2-.9 2-2V5c0-1.1-.9-2-2-2z" />
			<path d="M10 4v16" />
			<path d="m16 12-2-2 2-2" />
		</svg>
	{/if}
</button>

<!-- Sidebar -->
<div class="sidebar" class:visible>
	<div class="sidebar-content">
		<h3>Data Schema</h3>
		<div class="schema-container">
			{#if Object.keys(schema).length > 0}
				<div class="schema-grid">
					{#each Object.entries(schema) as [field, type]}
						<div class="schema-item">
							<span class="field-name">{field}</span>
							<span class="field-type">{type}</span>
						</div>
					{/each}
				</div>
			{:else}
				<p class="no-data">No schema available</p>
			{/if}
		</div>

		<h3>Context</h3>
		<div class="context-container">
			{#if isEditingContext}
				<div class="context-edit">
					<textarea
						value={editableContext}
						oninput={handleContextInput}
						placeholder="Enter context about your data..."
						rows="6"
					></textarea>
					<div class="context-actions">
						<button class="save-btn" onclick={onSaveContext}>Save</button>
						<button class="cancel-btn" onclick={onCancelEditContext}>Cancel</button>
					</div>
				</div>
			{:else}
				<div class="context-display">
					<p class="context-text">{context || 'No context available'}</p>
					<button class="edit-btn" onclick={onStartEditingContext}>Edit</button>
				</div>
			{/if}
		</div>
	</div>
</div>

<style>
	.sidebar-toggle {
		position: fixed;
		top: 50%;
		right: 10px;
		transform: translateY(-50%);
		z-index: 1001;
		background: #232f3e;
		color: white;
		border: none;
		border-radius: 50%;
		width: 40px;
		height: 40px;
		display: flex;
		align-items: center;
		justify-content: center;
		cursor: pointer;
		box-shadow: 0 2px 10px rgba(0, 0, 0, 0.2);
		transition: all 0.3s ease;
	}

	.sidebar-toggle:hover {
		background: #131a22;
		transform: translateY(-50%) scale(1.1);
	}

	.sidebar {
		position: fixed;
		top: 0;
		right: 0;
		width: 350px;
		height: 100vh;
		background: #f8f9fa;
		border-left: 1px solid #e9ecef;
		transform: translateX(100%);
		transition: transform 0.3s ease;
		z-index: 1000;
		overflow-y: auto;
	}

	.sidebar.visible {
		transform: translateX(0);
	}

	.sidebar-content {
		padding: 2rem;
	}

	.sidebar h3 {
		margin: 0 0 1rem 0;
		color: #232f3e;
		font-size: 1.2rem;
		font-weight: 600;
		border-bottom: 2px solid #ff9900;
		padding-bottom: 0.5rem;
	}

	.schema-container {
		margin-bottom: 2rem;
	}

	.schema-grid {
		display: flex;
		flex-direction: column;
		gap: 0.75rem;
	}

	.schema-item {
		display: flex;
		justify-content: space-between;
		align-items: center;
		padding: 0.75rem;
		background: white;
		border: 1px solid #e9ecef;
		border-radius: 8px;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
	}

	.field-name {
		font-weight: 600;
		color: #232f3e;
	}

	.field-type {
		background: #ff9900;
		color: white;
		padding: 0.25rem 0.5rem;
		border-radius: 4px;
		font-size: 0.875rem;
		font-weight: 500;
	}

	.context-container {
		background: white;
		border: 1px solid #e9ecef;
		border-radius: 8px;
		overflow: hidden;
	}

	.context-display {
		padding: 1rem;
	}

	.context-text {
		margin: 0 0 1rem 0;
		color: #495057;
		line-height: 1.5;
		min-height: 2rem;
	}

	.context-edit textarea {
		width: 100%;
		border: none;
		padding: 1rem;
		font-family: inherit;
		font-size: 0.9rem;
		resize: vertical;
		outline: none;
		background: #f8f9fa;
	}

	.context-actions {
		display: flex;
		gap: 0.5rem;
		padding: 0.75rem 1rem;
		background: #f8f9fa;
		border-top: 1px solid #e9ecef;
	}

	.edit-btn,
	.save-btn,
	.cancel-btn {
		padding: 0.5rem 1rem;
		border: none;
		border-radius: 4px;
		cursor: pointer;
		font-size: 0.875rem;
		font-weight: 500;
		transition: all 0.2s ease;
	}

	.edit-btn {
		background: #232f3e;
		color: white;
	}

	.edit-btn:hover {
		background: #131a22;
	}

	.save-btn {
		background: #28a745;
		color: white;
	}

	.save-btn:hover {
		background: #218838;
	}

	.cancel-btn {
		background: #6c757d;
		color: white;
	}

	.cancel-btn:hover {
		background: #5a6268;
	}

	.no-data {
		color: #6c757d;
		font-style: italic;
		margin: 0;
		padding: 1rem;
		text-align: center;
	}

	/* Responsive design */
	@media (max-width: 768px) {
		.sidebar {
			width: 100vw;
		}

		.sidebar-toggle {
			right: 15px;
		}
	}

	/* Scrollbar styling */
	.sidebar::-webkit-scrollbar {
		width: 6px;
	}

	.sidebar::-webkit-scrollbar-track {
		background: #f1f1f1;
		border-radius: 3px;
	}

	.sidebar::-webkit-scrollbar-thumb {
		background: #aab7b8;
		border-radius: 3px;
	}

	.sidebar::-webkit-scrollbar-thumb:hover {
		background: #8a9ba8;
	}
</style>
