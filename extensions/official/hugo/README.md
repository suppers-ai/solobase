# Hugo Extension

A powerful static site generator extension for Solobase, powered by Hugo.

## Features

- **Easy Site Creation**: Create new Hugo sites with a simple interface
- **Multiple Themes**: Support for various Hugo themes
- **File Management**: Built-in file browser and editor
- **One-Click Deploy**: Build and deploy sites with a single click
- **Live Preview**: Preview your site before publishing

## Requirements

- Hugo binary installed on the server
- File system access for site storage

## Installation

The Hugo extension is automatically registered when Solobase starts. To use it:

1. Ensure Hugo is installed on your system:
   ```bash
   # macOS
   brew install hugo

   # Linux (Debian/Ubuntu)
   sudo apt-get install hugo

   # Or download from https://gohugo.io/installation/
   ```

2. The extension will automatically create the necessary storage directories

## Usage

### Creating a New Site

1. Navigate to Admin → Extensions → Hugo
2. Click "New Site"
3. Fill in the site details:
   - Name: Your site name
   - Domain: Your site domain (optional)
   - Theme: Choose from available themes

### Editing Content

1. Click the "Edit" button on any site
2. Browse the file tree
3. Click on any file to edit it
4. Save your changes

### Building and Deploying

1. Click "Build & Deploy" on any site
2. Wait for the build to complete
3. View your site by clicking the "Preview" button

## API Endpoints

- `GET /admin/ext/hugo/sites` - List all sites
- `POST /admin/ext/hugo/sites` - Create a new site
- `GET /admin/ext/hugo/sites/{id}` - Get site details
- `DELETE /admin/ext/hugo/sites/{id}` - Delete a site
- `POST /admin/ext/hugo/sites/{id}/build` - Build a site
- `GET /admin/ext/hugo/sites/{id}/files` - List site files
- `POST /admin/ext/hugo/sites/{id}/files/read` - Read a file
- `POST /admin/ext/hugo/sites/{id}/files/save` - Save a file
- `GET /admin/ext/hugo/stats` - Get statistics

## Configuration

Default configuration can be found in `extensions/defaults/hugo.json`:

```json
{
  "hugo_binary_path": "hugo",
  "max_sites_per_user": 10,
  "max_site_size": 1073741824,
  "build_timeout": "10m",
  "allowed_themes": ["default", "blog", "portfolio"],
  "default_theme": "default",
  "storage_bucket": "hugo-sites"
}
```

## Storage Structure

Sites are stored in the following structure:
```
storage/ext/hugo/
├── sites/
│   └── {site-id}/
│       ├── config.toml
│       ├── content/
│       ├── layouts/
│       ├── static/
│       ├── themes/
│       └── public/ (build output)
└── public/
    └── {site-id}/ (deployed sites)
```

## Troubleshooting

### Hugo Binary Not Found

If you see "Hugo binary not found" errors:
1. Install Hugo using the instructions above
2. Update `hugo_binary_path` in the configuration to point to your Hugo binary

### Build Failures

If builds are failing:
1. Check Hugo version compatibility
2. Review build logs for specific errors
3. Ensure file permissions are correct
4. Verify theme compatibility

## Development

To extend or modify the Hugo extension:

1. Extension code is in `extensions/official/hugo/`
2. Main files:
   - `extension.go` - Extension registration and lifecycle
   - `handlers.go` - HTTP request handlers
   - `services.go` - Hugo operations (build, file management)
   - `models.go` - Data models

## License

MIT License - see LICENSE file for details
