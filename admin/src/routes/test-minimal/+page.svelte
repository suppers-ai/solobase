<script>
	// No imports at all
</script>

<div style="padding: 2rem;">
	<h1>Minimal Test - No CSS Imports</h1>
	
	<div style="margin-bottom: 1rem;">
		<label>Plain input with no classes:</label>
		<input type="text" value="Can you select this?" style="width: 100%; padding: 0.5rem;" />
	</div>
	
	<div style="margin-bottom: 1rem;">
		<label>Input with explicit user-select:</label>
		<input 
			type="text" 
			value="Can you select this?" 
			style="width: 100%; padding: 0.5rem; user-select: text !important; -webkit-user-select: text !important;" 
		/>
	</div>
	
	<div style="margin-bottom: 1rem;">
		<label>Regular div text for comparison:</label>
		<div style="padding: 0.5rem; border: 1px solid #ccc;">Can you select this text in a div?</div>
	</div>
</div>