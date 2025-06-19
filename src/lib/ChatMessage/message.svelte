<script lang="ts">
	interface Message {
		id: number;
		type: 'user' | 'assistant';
		content: string;
		timestamp: Date;
	}

	interface Props {
		message: Message;
	}

	let { message }: Props = $props();
</script>

<div class="message {message.type}">
	<div class="message-avatar">
		{#if message.type === 'assistant'}
			<div class="avatar buzz-avatar">
				<svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor"> </svg>
			</div>
		{:else}
			<div class="avatar user-avatar">
				<svg
					width="20"
					height="20"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
				>
					<path d="M20 21v-2a4 4 0 0 0-4-4H8a4 4 0 0 0-4 4v2" />
					<circle cx="12" cy="7" r="4" />
				</svg>
			</div>
		{/if}
	</div>
	<div class="message-content">
		<div class="message-text">
			{message.content}
		</div>
		<div class="message-time">
			{message.timestamp.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}
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

	.chat-container {
		display: flex;
		flex-direction: column;
		height: 100vh;
		max-width: 1200px;
		margin: 0 auto;
		background: #ffffff;
		border-radius: 0;
		box-shadow: 0 0 20px rgba(0, 0, 0, 0.1);
		position: relative;
	}

	/* Processing indicator */
	.processing-indicator {
		color: #ff9900;
		font-weight: 500;
		animation: processingPulse 2s infinite ease-in-out;
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

	.chat-header {
		background: linear-gradient(90deg, #232f3e 0%, #37475a 100%);
		color: white;
		padding: 1rem 1.5rem;
		border-bottom: 1px solid #d5dbdb;
		flex-shrink: 0;
	}

	.header-content {
		display: flex;
		justify-content: space-between;
		align-items: center;
	}

	.header-left {
		display: flex;
		align-items: center;
		gap: 1rem;
	}

	.buzz-avatar {
		width: 40px;
		height: 40px;
		background: #ff9900;
		color: #000000;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		font-weight: 700;
	}

	.clear-btn {
		background: rgba(255, 255, 255, 0.1);
		border: 1px solid rgba(255, 255, 255, 0.2);
		color: rgba(255, 255, 255, 0.8);
		padding: 0.5rem;
		border-radius: 6px;
		cursor: pointer;
		transition: all 0.2s ease;
		display: flex;
		align-items: center;
		justify-content: center;
	}

	.clear-btn:hover {
		background: rgba(255, 255, 255, 0.2);
		color: #ffffff;
		border-color: rgba(255, 255, 255, 0.3);
	}

	.chat-main {
		flex: 1;
		overflow: hidden;
		background: #fafafa;
	}

	.messages-container {
		height: 100%;
		overflow-y: auto;
		padding: 1.5rem;
		display: flex;
		flex-direction: column;
		gap: 1.5rem;
	}

	.message {
		display: flex;
		gap: 0.75rem;
		max-width: 800px;
		animation: slideIn 0.3s ease-out;
	}

	.message.user {
		align-self: flex-end;
		flex-direction: row-reverse;
	}

	.message-avatar {
		flex-shrink: 0;
	}

	.avatar {
		width: 32px;
		height: 32px;
		border-radius: 50%;
		display: flex;
		align-items: center;
		justify-content: center;
		border: 2px solid;
	}

	.avatar.buzz-avatar {
		background: #ff9900;
		color: #000000;
		border-color: #ff9900;
	}

	.avatar.user-avatar {
		background: #146eb4;
		color: #ffffff;
		border-color: #146eb4;
	}

	.message-content {
		flex: 1;
		min-width: 0;
	}

	.message-text {
		background: #ffffff;
		padding: 1rem 1.25rem;
		border-radius: 12px;
		border: 1px solid #d5dbdb;
		line-height: 1.6;
		color: #0f1419;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
		font-size: 0.95rem;
		white-space: pre-wrap;
	}

	.message.user .message-text {
		background: #146eb4;
		color: #ffffff;
		border-color: #146eb4;
	}

	.message-time {
		font-size: 0.75rem;
		color: #666;
		margin-top: 0.5rem;
		padding: 0 0.25rem;
	}

	.message.user .message-time {
		text-align: right;
	}

	.typing-indicator {
		background: #ffffff;
		padding: 1rem 1.25rem;
		border-radius: 12px;
		border: 1px solid #d5dbdb;
		box-shadow: 0 1px 3px rgba(0, 0, 0, 0.1);
	}

	.typing-dots {
		display: flex;
		gap: 0.25rem;
		align-items: center;
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

	.input-footer {
		text-align: center;
		margin-top: 0.75rem;
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
		.chat-container {
			height: 100vh;
			border-radius: 0;
		}

		.chat-header {
			padding: 1rem;
		}

		.messages-container {
			padding: 1rem;
		}

		.message {
			max-width: 100%;
		}

		.chat-footer {
			padding: 1rem;
		}

		.rocket {
			font-size: 4rem;
		}

		.infinity-text {
			font-size: 1.5rem;
		}

		.star {
			font-size: 1.5rem;
		}
	}

	/* Scrollbar styling */
	.messages-container::-webkit-scrollbar {
		width: 6px;
	}

	.messages-container::-webkit-scrollbar-track {
		background: #f1f1f1;
		border-radius: 3px;
	}

	.messages-container::-webkit-scrollbar-thumb {
		background: #aab7b8;
		border-radius: 3px;
	}

	.messages-container::-webkit-scrollbar-thumb:hover {
		background: #8a9ba8;
	}
</style>
