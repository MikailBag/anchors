use anyhow::{bail, Context as _, Result};
use serde_yaml::Value;
use std::{collections::HashMap, path::Path};

fn main() -> anyhow::Result<()> {
    let args = clap::App::new("anchors")
        .arg(
            clap::Arg::with_name("templates-dir")
                .takes_value(true)
                .help("path to templates dir")
                .long_help(
                    "each yaml file in this dir will be expanded \
                     to Github Actions workflow",
                )
                .required(true),
        )
        .arg(
            clap::Arg::with_name("modify")
                .short("m")
                .long("modify")
                .help("Write expanded workflows")
                .long_help(
                    "by default anchors will only check that .github/workflows is \
    up to date. With this flag set, it will update this directory instead",
                ),
        )
        .get_matches();
    let templates_dir = Path::new(args.value_of_os("templates-dir").unwrap());
    if !templates_dir.exists() {
        bail!("templates dir does not exist");
    }
    verify_current_dir()?;
    let templates = load_templates_data(templates_dir)?;
    let expanded = expand_templates(&templates)?;
    if args.is_present("modify") {
        remove_old_workflows().context("failed to delete old workflows")?;
        for (name, value) in expanded {
            println!("Emitting {}", name);
            let value = serde_yaml::to_vec(&value)?;
            let path = Path::new(".github/workflows").join(format!("{}.yaml", name));
            std::fs::write(path, value)?;
        }
    } else {
        compare(&expanded)?;
    }
    Ok(())
}

/// Checks that files in .github/workflows match `expanded`
fn compare(expanded: &HashMap<String, Value>) -> Result<()> {
    for (name, value) in expanded {
        let workflow_path = Path::new(".github/workflows").join(format!("{}.yaml", name));
        if !workflow_path.exists() {
            bail!("File {} not exists", workflow_path.display());
        }
        let actual_content = std::fs::read(&workflow_path)?;
        let actual_content: Value =
            serde_yaml::from_slice(&actual_content).context("file contains non-utf8 data")?;
        if actual_content != *value {
            bail!("File {} is outdated", workflow_path.display());
        }
    }
    for item in std::fs::read_dir(".github/workflows")? {
        let item = item?;
        let path = item.path();
        let item = path
            .file_stem()
            .and_then(|os_str| os_str.to_str())
            .ok_or_else(|| anyhow::anyhow!("Bad workflow file name"))?;
        if !expanded.contains_key(item) {
            bail!("File {} should not exist", path.display());
        }
    }
    Ok(())
}

fn remove_old_workflows() -> Result<()> {
    std::fs::remove_dir_all(".github/workflows")?;
    std::fs::create_dir(".github/workflows")?;
    Ok(())
}

fn verify_current_dir() -> Result<()> {
    let path = Path::new(".github/workflows");
    if !path.exists() {
        bail!(
            "It seems that current dir is not repository root: \
        .github/workflows directory missing"
        )
    }
    Ok(())
}

struct TemplatesData {
    workflows: HashMap<String, Value>,
    blocks: HashMap<String, Value>,
}

fn load_yamls(path: &Path) -> Result<HashMap<String, Value>> {
    let mut values = HashMap::new();
    let dir_iter =
        std::fs::read_dir(path).with_context(|| format!("dir {} not readable", path.display()))?;
    for item in dir_iter {
        let item = item?;
        let path = item.path();
        if path.extension() != Some(std::ffi::OsStr::new("yaml")) {
            continue;
        }
        let name = path
            .file_stem()
            .ok_or_else(|| anyhow::anyhow!("path {} does contain name", path.display()))?;
        let name = name
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("file name is not utf8: {}", path.display()))?;
        let data = std::fs::read(&path)
            .with_context(|| format!("path {} not readable", path.display()))?;
        let data: Value = serde_yaml::from_slice(&data)
            .with_context(|| format!("file {} is not valid YAML", path.display()))?;
        values.insert(name.to_string(), data);
    }
    Ok(values)
}

fn load_templates_data(path: &Path) -> Result<TemplatesData> {
    let workflows = load_yamls(path)?;
    let blocks = load_yamls(&path.join("blocks"))?;
    let templates = TemplatesData { workflows, blocks };

    Ok(templates)
}

fn expand_templates(templates: &TemplatesData) -> Result<HashMap<String, Value>> {
    let mut expanded = HashMap::new();

    for (name, workflow) in &templates.workflows {
        println!("Expanding {}", name);
        let expanded_workflow = expand_value(workflow, &templates.blocks)
            .with_context(|| format!("Error in workflow {}", name))?;
        expanded.insert(name.clone(), expanded_workflow);
    }
    Ok(expanded)
}

fn expand_value(value: &serde_yaml::Value, blocks: &HashMap<String, Value>) -> Result<Value> {
    if let Ok(action) = serde_yaml::from_value::<TemplateAction>(value.clone()) {
        match blocks.get(&action.include) {
            Some(block) => Ok(block.clone()),
            None => bail!("unknown $include: {}", action.include),
        }
    } else {
        match value {
            Value::Mapping(mapping) => {
                let mut expanded = serde_yaml::Mapping::new();
                for (k, v) in mapping {
                    let v = expand_value(v, blocks)?;
                    expanded.insert(k.clone(), v);
                }
                Ok(Value::Mapping(expanded))
            }
            Value::Sequence(seq) => {
                let mut expanded = Vec::new();
                for v in seq {
                    let v = expand_value(v, blocks)?;
                    expanded.push(v);
                }
                Ok(Value::Sequence(expanded))
            }
            Value::String(..) | Value::Number(..) | Value::Bool(..) | Value::Null => {
                Ok(value.clone())
            }
        }
    }
}

#[derive(serde::Deserialize)]
struct TemplateAction {
    #[serde(rename = "$include")]
    include: String,
}
