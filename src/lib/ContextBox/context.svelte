<script lang="ts">
	interface Props {
		contextText?: string;
		disabled?: boolean;
		onContextChange?: (context: string) => void;
	}

	let { contextText = '', disabled = false, onContextChange }: Props = $props();

	function handleInput(event: Event) {
		const target = event.target as HTMLTextAreaElement;
		const newValue = target.value;
		contextText = newValue;
		onContextChange?.(newValue);
	}
</script>

<div class="context-section">
	<h3 class="context-title">Data Context</h3>
	<p class="context-description">
		Provide additional context about your data to help improve query understanding and results.
	</p>

	<div class="context-input-wrapper">
		<textarea
			bind:value={contextText}
			oninput={handleInput}
			{disabled}
			placeholder="Describe your dataset and what it's used for. This will help improve your output"
			class="context-textarea"
			rows="4"
		></textarea>

		<div class="character-count">
			{contextText.length} / 1000 characters
		</div>
	</div>
</div>

<style>
	.context-section {
		margin: 2rem 0;
		padding: 1.5rem;
		border: 1px solid #e1e8ed;
		border-radius: 6px;
		background-color: #f8f9fa;
	}

	.context-title {
		margin: 0 0 0.5rem 0;
		font-size: 1.1rem;
		font-weight: 600;
		color: #232f3e;
	}

	.context-description {
		margin: 0 0 1rem 0;
		font-size: 0.9rem;
		color: #5a6c75;
		line-height: 1.4;
	}

	.context-input-wrapper {
		position: relative;
	}

	.context-textarea {
		width: 100%;
		min-height: 120px;
		padding: 0.875rem;
		border: 1px solid #d5dbdb;
		border-radius: 4px;
		font-family: inherit;
		font-size: 0.9rem;
		line-height: 1.5;
		resize: vertical;
		transition:
			border-color 0.2s ease,
			box-shadow 0.2s ease;
		background-color: #ffffff;
	}

	.context-textarea:focus {
		outline: none;
		border-color: #ff9900;
		box-shadow: 0 0 0 2px rgba(255, 153, 0, 0.1);
	}

	.context-textarea:disabled {
		background-color: #f5f5f5;
		color: #999;
		cursor: not-allowed;
		opacity: 0.7;
	}

	.context-textarea::placeholder {
		color: #aab7b8;
		font-style: italic;
	}

	.character-count {
		position: absolute;
		bottom: 0.5rem;
		right: 0.75rem;
		font-size: 0.75rem;
		color: #aab7b8;
		background-color: rgba(255, 255, 255, 0.9);
		padding: 0.25rem 0.5rem;
		border-radius: 3px;
		pointer-events: none;
	}

	@media (max-width: 768px) {
		.context-section {
			margin: 1.5rem 0;
			padding: 1rem;
		}

		.context-textarea {
			min-height: 100px;
		}
	}
</style>
