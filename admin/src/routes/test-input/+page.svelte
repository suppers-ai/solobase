<script>
	import '../../app.css';
</script>

<div class="test-container">
	<h1>Input Selection Test</h1>
	
	<div class="test-group">
		<h2>Plain HTML Input</h2>
		<input type="text" value="Try to select this text" />
	</div>
	
	<div class="test-group">
		<h2>Input with Tailwind Classes</h2>
		<input 
			type="text" 
			class="w-full px-3 py-2 border border-gray-300 rounded-md" 
			value="Try to select this text with Tailwind classes" 
		/>
	</div>
	
	<div class="test-group">
		<h2>Textarea</h2>
		<textarea rows="3">Try to select this text in a textarea</textarea>
	</div>
	
	<div class="test-group">
		<h2>Input with form-input class</h2>
		<input 
			type="text" 
			class="form-input" 
			value="Try to select this text with form-input class" 
		/>
	</div>
</div>

<style>
	.test-container {
		padding: 2rem;
		max-width: 600px;
		margin: 0 auto;
	}
	
	.test-group {
		margin-bottom: 2rem;
	}
	
	h1 {
		font-size: 1.5rem;
		font-weight: bold;
		margin-bottom: 2rem;
	}
	
	h2 {
		font-size: 1rem;
		font-weight: 600;
		margin-bottom: 0.5rem;
	}
	
	input, textarea {
		width: 100%;
		padding: 0.5rem;
		border: 1px solid #ccc;
		border-radius: 4px;
	}
</style>