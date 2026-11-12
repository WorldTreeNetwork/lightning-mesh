<script lang="ts">
	let {
		scanning = false,
		onclick
	}: {
		scanning?: boolean;
		onclick?: () => void;
	} = $props();
</script>

<button
	type="button"
	class="radar"
	class:scanning
	aria-label={scanning ? 'Scanning' : 'Scan'}
	aria-busy={scanning}
	onclick={() => {
		if (!scanning) onclick?.();
	}}
>
	<svg class="disc" viewBox="0 0 200 200" aria-hidden="true">
		<circle class="ring" cx="100" cy="100" r="86" />
		<circle class="ring" cx="100" cy="100" r="56" />
		<circle class="ring" cx="100" cy="100" r="28" />
		<line class="cross" x1="100" y1="14" x2="100" y2="186" />
		<line class="cross" x1="14" y1="100" x2="186" y2="100" />
		<g class="arm">
			<path class="wedge" d="M100 100 L100 14 A86 86 0 0 1 180.8 70.6 Z" />
			<line class="beam" x1="100" y1="100" x2="100" y2="14" />
		</g>
		<circle class="hub" cx="100" cy="100" r="7" />
		<circle class="blip" style="--delay: -0.12s" cx="112" cy="62" r="6" />
		<circle class="blip" style="--delay: -0.67s" cx="156" cy="122" r="6" />
		<circle class="blip" style="--delay: -1.22s" cx="72" cy="164" r="6.5" />
		<circle class="blip" style="--delay: -1.83s" cx="54" cy="74" r="5" />
	</svg>
</button>

<style>
	.radar {
		display: grid;
		place-items: center;
		width: 36px;
		height: 36px;
		padding: 0;
		border: 1px solid var(--border-strong);
		border-radius: 50%;
		background: var(--surface);
		color: var(--accent);
		cursor: pointer;
	}

	.radar:hover {
		background: var(--row-hover);
	}

	.radar:focus-visible {
		outline: 2px solid var(--focus);
		outline-offset: 2px;
	}

	.scanning {
		cursor: default;
	}

	.disc {
		display: block;
		width: 26px;
		height: 26px;
	}

	.ring,
	.cross {
		fill: none;
		stroke: currentColor;
		stroke-width: 6;
		opacity: 0.55;
	}

	.cross {
		opacity: 0.28;
	}

	.arm {
		transform-origin: 100px 100px;
	}

	.scanning .arm {
		animation: radar-sweep 1.4s linear infinite;
	}

	.wedge {
		fill: currentColor;
		opacity: 0.28;
	}

	.beam {
		stroke: currentColor;
		stroke-width: 8;
		stroke-linecap: round;
	}

	.hub {
		fill: currentColor;
	}

	.blip {
		fill: var(--ok);
		opacity: 0.35;
	}

	.scanning .blip {
		animation: radar-blip 1.4s linear infinite;
		animation-delay: var(--delay);
	}

	@keyframes radar-sweep {
		to {
			transform: rotate(360deg);
		}
	}

	@keyframes radar-blip {
		0% {
			opacity: 1;
		}

		18% {
			opacity: 0.3;
		}

		100% {
			opacity: 0.25;
		}
	}

	@media (prefers-reduced-motion: reduce) {
		.scanning .arm,
		.scanning .blip {
			animation: none;
		}

		.blip {
			opacity: 0.8;
		}
	}
</style>
