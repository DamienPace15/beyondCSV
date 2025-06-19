<script lang="ts">
	import { onMount } from 'svelte';

	interface Props {
		value: string;
		isDisabled: boolean;
		isParquetReady: boolean;
		onSend: () => void;
		onKeydown: (event: KeyboardEvent) => void;
		onValueChange: (value: string) => void;
	}

	let {
		value = '',
		isDisabled = false,
		isParquetReady = false,
		onSend,
		onKeydown,
		onValueChange
	}: Props = $props();

	let messageInput: HTMLTextAreaElement;

	// Reactive assignment to keep parent in sync
	$effect(() => {
		onValueChange(value);
	});

	onMount(() => {
		messageInput?.focus();
	});
</script>

<footer class="chat-footer">
	<div class="input-container">
		<div class="input-group">
			<textarea
				bind:this={messageInput}
				bind:value
				onkeydown={onKeydown}
				placeholder={isParquetReady
					? "Ask Buzz about your dataset... (psst: try 'to infinity and beyond!')"
					: 'Please wait while your data is being processed...'}
				class="message-input"
				rows="1"
				disabled={isDisabled}
			></textarea>
			<!-- svelte-ignore a11y_consider_explicit_label -->
			<button
				class="send-button"
				onclick={onSend}
				disabled={!value.trim() || isDisabled}
				title="Send message (Enter)"
			>
				<svg
					width="20"
					height="20"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
				>
					<line x1="22" y1="2" x2="11" y2="13" />
					<polygon points="22,2 15,22 11,13 2,9 22,2" />
				</svg>
			</button>
		</div>
	</div>
</footer>

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

	@keyframes processingPulse {
		0%,
		100% {
			opacity: 1;
		}
		50% {
			opacity: 0.6;
		}
	}

	.chat-footer {
		background: #ffffff;
		border-top: 1px solid #d5dbdb;
		padding: 1.5rem;
		flex-shrink: 0;
	}

	.input-container {
		max-width: 800px;
		margin: 0 auto;
	}

	.input-group {
		display: flex;
		gap: 0.75rem;
		align-items: flex-end;
		background: #ffffff;
		border: 2px solid #d5dbdb;
		border-radius: 12px;
		padding: 0.75rem;
		transition: all 0.2s ease;
	}

	.input-group:focus-within {
		border-color: #ff9900;
		box-shadow: 0 0 0 3px rgba(255, 153, 0, 0.1);
	}

	.message-input {
		flex: 1;
		border: none;
		outline: none;
		resize: none;
		font-family: inherit;
		font-size: 0.95rem;
		line-height: 1.5;
		color: #0f1419;
		background: transparent;
		min-height: 24px;
		max-height: 150px;
		overflow-y: auto;
	}

	.message-input::placeholder {
		color: #aab7b8;
	}

	.message-input:disabled {
		opacity: 0.6;
		cursor: not-allowed;
	}

	.send-button {
		background: #ff9900;
		color: #000000;
		border: none;
		border-radius: 8px;
		padding: 0.5rem;
		cursor: pointer;
		transition: all 0.2s ease;
		display: flex;
		align-items: center;
		justify-content: center;
		flex-shrink: 0;
	}

	.send-button:hover:not(:disabled) {
		background: #e88b00;
		transform: translateY(-1px);
		box-shadow: 0 2px 4px rgba(0, 0, 0, 0.1);
	}

	.send-button:disabled {
		background: #aab7b8;
		color: #5a6c75;
		cursor: not-allowed;
		opacity: 0.6;
	}

	@keyframes slideIn {
		from {
			opacity: 0;
			transform: translateY(10px);
		}
		to {
			opacity: 1;
			transform: translateY(0);
		}
	}

	@keyframes typingDot {
		0%,
		60%,
		100% {
			transform: scale(1);
			opacity: 0.5;
		}
		30% {
			transform: scale(1.2);
			opacity: 1;
		}
	}

	/* Responsive design */
	@media (max-width: 768px) {
		.chat-footer {
			padding: 1rem;
		}
	}
</style>
