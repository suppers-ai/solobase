# Solobase Demo Resources

This folder contains all demo-related resources for Solobase, including deployment configurations and example code.

## Structure

- **`deployment/`** - Demo deployment configurations
  - `README.md` - Deployment instructions for the demo environment
  - `fly.toml` - Fly.io configuration for demo deployment
  - `Dockerfile` - Docker configuration for demo container

- **`code/`** - Demo code and examples
  - `demo_setup.go` - Demo data setup for IAM (roles, users, policies)

## Usage

### Deploying the Demo

See `deployment/README.md` for detailed instructions on deploying the Solobase demo.

### Demo Data

The `code/demo_setup.go` file contains functions to set up demo data for testing IAM features. This includes:
- Sample roles (admin, user, viewer)
- Sample users with different permission levels
- Example policies for role-based access control

## Note

These resources are for demonstration and testing purposes only. Do not use demo configurations or data in production environments.