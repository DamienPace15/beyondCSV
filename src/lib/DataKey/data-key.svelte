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
					💡 <strong>When in doubt:</strong> DateTime is usually the safest choice for most business
					data
				</div>
			</div>
		</div>
	</div>
</details>

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
</style>
