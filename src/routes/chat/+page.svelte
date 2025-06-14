<!-- +page.svelte -->
<script lang="ts">
	import { onMount } from 'svelte';
	import { tick } from 'svelte';
	import { generateResponseFromMessage } from './queryData';
	import { page } from '$app/stores';
	import type { LayoutData } from './$types';

	// Type definitions
	interface Message {
		id: number;
		type: 'user' | 'assistant';
		content: string;
		timestamp: Date;
	}

	interface ApiResponse {
		statusCode: number;
		response_message: string;
	}

	let { data }: { data: LayoutData } = $props();
	let key: string | null = $derived($page.url.searchParams.get('key'));

	let messages: Message[] = $state([
		{
			id: 1,
			type: 'assistant',
			content: 'Hello! Ask Buzz anything about your data that you uploaded!',
			timestamp: new Date()
		}
	]);

	let currentMessage: string = $state('');
	let isTyping: boolean = $state(false);
	let chatContainer: HTMLDivElement;
	let messageInput: HTMLTextAreaElement;
	let showEasterEgg: boolean = $state(false);

	// Auto-scroll to bottom when new messages are added
	$effect(() => {
		if (chatContainer && messages.length > 0) {
			tick().then(() => {
				chatContainer.scrollTop = chatContainer.scrollHeight;
			});
		}
	});

	function triggerBuzzLightyearEasterEgg(): void {
		showEasterEgg = true;

		const buzzResponse: Message = {
			id: Date.now() + 2,
			type: 'assistant',
			content:
				"🚀 TO INFINITY AND BEYOND! 🚀\n\nYou found the secret! I may be a data assistant, but I've got the heart of a Space Ranger! Now, what can this space-age AI help you discover in your data? The universe of insights awaits!",
			timestamp: new Date()
		};

		messages = [...messages, buzzResponse];

		setTimeout(() => {
			showEasterEgg = false;
		}, 3000);
	}

	async function sendMessage(): Promise<void> {
		if (!currentMessage.trim() || isTyping) return;

		// Check for easter egg trigger
		const message = currentMessage.trim().toLowerCase();
		const isEasterEggTrigger =
			message.includes('to infinity and beyond') ||
			message.includes('infinity and beyond') ||
			message === 'to infinity and beyond!' ||
			message === 'infinity and beyond!';

		const userMessage: Message = {
			id: Date.now(),
			type: 'user',
			content: currentMessage.trim(),
			timestamp: new Date()
		};

		messages = [...messages, userMessage];
		const messageToSend: string = currentMessage;
		currentMessage = '';

		// Trigger easter egg if detected
		if (isEasterEggTrigger) {
			triggerBuzzLightyearEasterEgg();
			return;
		}

		isTyping = true;

		try {
			const responseContent: string = await generateResponse(messageToSend);

			const aiResponse: Message = {
				id: Date.now() + 1,
				type: 'assistant',
				content: responseContent,
				timestamp: new Date()
			};

			messages = [...messages, aiResponse];
		} catch (error) {
			console.error('Error generating response:', error);

			const errorResponse: Message = {
				id: Date.now() + 1,
				type: 'assistant',
				content: 'Sorry, I encountered an error while processing your request. Please try again.',
				timestamp: new Date()
			};

			messages = [...messages, errorResponse];
		} finally {
			isTyping = false;
		}
	}

	async function generateResponse(userMessage: string): Promise<string> {
		if (!key) {
			throw new Error('No key provided');
		}

		const responses: ApiResponse = await generateResponseFromMessage(
			data.env.CORE_API_URL!,
			userMessage,
			key
		);

		return responses.response_message;
	}

	function handleKeydown(event: KeyboardEvent): void {
		if (event.key === 'Enter' && !event.shiftKey) {
			event.preventDefault();
			sendMessage();
		}
	}

	function clearChat(): void {
		messages = [
			{
				id: 1,
				type: 'assistant',
				content: "Hello! I'm Buzz,. How can I help you today?",
				timestamp: new Date()
			}
		];
	}

	onMount(() => {
		messageInput?.focus();
	});
</script>

<div class="chat-container">
	<!-- Easter Egg Overlay -->
	{#if showEasterEgg}
		<div class="easter-egg-overlay">
			<div class="buzz-lightyear-animation">
				<div class="rocket">🚀</div>
				<div class="stars">
					<div class="star">⭐</div>
					<div class="star">✨</div>
					<div class="star">⭐</div>
					<div class="star">✨</div>
					<div class="star">⭐</div>
				</div>
				<div class="infinity-text">TO INFINITY AND BEYOND!</div>
			</div>
		</div>
	{/if}

	<header class="chat-header">
		<div class="header-content">
			<div class="header-left">
				<div class="claude-avatar">
					<svg width="24" height="24" viewBox="0 0 24 24" fill="currentColor"> </svg>
				</div>
				<div class="header-info">
					<h1>Buzz</h1>
					<p class="subtitle">
						The natural language query assistant that can help with your data and beyond
					</p>
				</div>
			</div>
			<!-- svelte-ignore a11y_consider_explicit_label -->
			<button class="clear-btn" onclick={clearChat} title="Clear conversation">
				<svg
					width="20"
					height="20"
					viewBox="0 0 24 24"
					fill="none"
					stroke="currentColor"
					stroke-width="2"
				>
					<path
						d="M3 6h18M19 6v14c0 1-1 2-2 2H7c-1 0-2-1-2-2V6m3 0V4c0-1 1-2 2-2h4c0 1 1 2 2 2v2"
					/>
					<line x1="10" y1="11" x2="10" y2="17" />
					<line x1="14" y1="11" x2="14" y2="17" />
				</svg>
			</button>
		</div>
	</header>

	<main class="chat-main">
		<div class="messages-container" bind:this={chatContainer}>
			{#each messages as message (message.id)}
				<div class="message {message.type}">
					<div class="message-avatar">
						{#if message.type === 'assistant'}
							<div class="avatar claude-avatar">
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
			{/each}

			{#if isTyping}
				<div class="message assistant">
					<div class="message-avatar">
						<div class="avatar claude-avatar">
							<svg width="20" height="20" viewBox="0 0 24 24" fill="currentColor"> </svg>
						</div>
					</div>
					<div class="message-content">
						<div class="typing-indicator">
							<div class="typing-dots">
								<span></span>
								<span></span>
								<span></span>
							</div>
						</div>
					</div>
				</div>
			{/if}
		</div>
	</main>

	<footer class="chat-footer">
		<div class="input-container">
			<div class="input-group">
				<textarea
					bind:this={messageInput}
					bind:value={currentMessage}
					onkeydown={handleKeydown}
					placeholder="Ask Buzz about your dataset... (psst: try 'to infinity and beyond!')"
					class="message-input"
					rows="1"
					disabled={isTyping}
				></textarea>
				<!-- svelte-ignore a11y_consider_explicit_label -->
				<button
					class="send-button"
					onclick={sendMessage}
					disabled={!currentMessage.trim() || isTyping}
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

	/* Easter Egg Styles */
	.easter-egg-overlay {
		position: fixed;
		top: 0;
		left: 0;
		width: 100vw;
		height: 100vh;
		background: linear-gradient(45deg, #1a1a2e, #16213e, #0f3460);
		z-index: 1000;
		display: flex;
		align-items: center;
		justify-content: center;
		animation: easterEggFadeIn 0.5s ease-out;
	}

	.buzz-lightyear-animation {
		text-align: center;
		position: relative;
	}

	.rocket {
		font-size: 8rem;
		animation: rocketFly 2s ease-in-out;
		display: block;
		margin-bottom: 2rem;
	}

	.stars {
		position: absolute;
		top: 0;
		left: 0;
		width: 100%;
		height: 100%;
		pointer-events: none;
	}

	.star {
		position: absolute;
		font-size: 2rem;
		animation: starTwinkle 1.5s infinite ease-in-out;
	}

	.star:nth-child(1) {
		top: 20%;
		left: 10%;
		animation-delay: 0s;
	}

	.star:nth-child(2) {
		top: 30%;
		right: 15%;
		animation-delay: 0.3s;
	}

	.star:nth-child(3) {
		bottom: 40%;
		left: 20%;
		animation-delay: 0.6s;
	}

	.star:nth-child(4) {
		bottom: 30%;
		right: 25%;
		animation-delay: 0.9s;
	}

	.star:nth-child(5) {
		top: 50%;
		left: 50%;
		animation-delay: 1.2s;
	}

	.infinity-text {
		font-size: 3rem;
		font-weight: bold;
		color: #00ff88;
		text-shadow:
			0 0 20px #00ff88,
			0 0 40px #00ff88;
		animation: infinityGlow 1s ease-in-out infinite alternate;
		letter-spacing: 0.1em;
	}

	@keyframes easterEggFadeIn {
		from {
			opacity: 0;
			transform: scale(0.8);
		}
		to {
			opacity: 1;
			transform: scale(1);
		}
	}

	@keyframes rocketFly {
		0% {
			transform: translateX(-100vw) rotate(-45deg);
		}
		50% {
			transform: translateX(0) rotate(0deg);
		}
		100% {
			transform: translateX(0) rotate(0deg) scale(1.2);
		}
	}

	@keyframes starTwinkle {
		0%,
		100% {
			opacity: 0.3;
			transform: scale(1);
		}
		50% {
			opacity: 1;
			transform: scale(1.3);
		}
	}

	@keyframes infinityGlow {
		0% {
			text-shadow:
				0 0 20px #00ff88,
				0 0 40px #00ff88;
		}
		100% {
			text-shadow:
				0 0 30px #00ff88,
				0 0 60px #00ff88,
				0 0 80px #00ff88;
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

	.claude-avatar {
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

	.header-info h1 {
		margin: 0;
		font-size: 1.5rem;
		font-weight: 700;
		color: #ffffff;
		letter-spacing: -0.025em;
	}

	.header-info .subtitle {
		margin: 0;
		font-size: 0.9rem;
		color: rgba(255, 255, 255, 0.8);
		font-weight: 300;
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

	.avatar.claude-avatar {
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

	.typing-dots span {
		width: 8px;
		height: 8px;
		border-radius: 50%;
		background: #aab7b8;
		animation: typingDot 1.4s infinite ease-in-out;
	}

	.typing-dots span:nth-child(2) {
		animation-delay: 0.2s;
	}

	.typing-dots span:nth-child(3) {
		animation-delay: 0.4s;
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

		.header-info h1 {
			font-size: 1.25rem;
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
