# Anchors
`Anchors` is tool that brings basic code reusage to [Github Actions](https://github.com/features/actions)
## Installation
### Cargo
```bash
# Install: 
cargo install --git https://github.com/mikailbag/anchors
# Use:
anchors
```
### Docker
```bash
# Use
docker run --interactive --rm https://docker.pkg.github.com/mikailbag/anchors/anchors:master -v $actions_dir:/tpl:ro -v `pwd`/.github/workflows:/.github/workflows 
```
## Usage
Create new directory in repo, which will contain your workflows. In this example, it will be `ci/workflows`.
So, we want `Anchors` to generate `.github/workflows` from `ci/workflows`.
Execute:
```bash
cd repository_root
anchors ci/workflows
```

It is recommended to execute following command in CI (also in repository root):
```bash
anchors --exit-code ci/workflows
```
`Anchors` will fail if `ci/workflows` does not match `.github/workflows` (it can happen if someone changes `ci/workflows` but has not executed `anchors`).

## Template format

Again, it is assumed that `ci/workflows` is *template dir*.
Each file in this directory will become workflow (i.e. it will be mapped to file in `./github/workflows` with same name).
Anchors templates are compatible with Actions workflows, but additional features are available.

Includes:
```yaml
# ...
jobs:
  standard_job:
    # ...
  job_with_included_steps:
    steps:
      - uses: action1
      - uses: action2
      # this will include ci/workflows/blocks/step3.yaml
      - $include: step3
  # we can include whole job also (and any other element, too: anchors work on YAML level)
  included_job:
    $include: some_job
```