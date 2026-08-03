use std::collections::HashMap;

#[derive(Clone, Debug)]
pub struct Environment {
    pub name: String,
    pub variables: HashMap<String, String>,
}

impl Environment {
    pub fn get(&self, key: &str) -> Option<&str> {
        self.variables.get(key).map(|s| s.as_str())
    }
}

#[derive(Clone, Debug)]
pub struct EnvironmentStore {
    pub environments: Vec<Environment>,
    pub active_name: Option<String>,
}

impl EnvironmentStore {
    pub fn new() -> Self {
        Self {
            environments: vec![],
            active_name: None,
        }
    }

    pub fn active(&self) -> Option<&Environment> {
        self.active_name
            .as_ref()
            .and_then(|name| self.environments.iter().find(|e| &e.name == name))
    }

    pub fn active_mut(&mut self) -> Option<&mut Environment> {
        let name = self.active_name.clone()?;
        self.environments.iter_mut().find(|e| e.name == name)
    }

    pub fn resolve(&self, input: &str) -> String {
        let Some(env) = self.active() else {
            return input.to_string();
        };
        let mut out = input.to_string();
        for (key, value) in &env.variables {
            out = out.replace(&format!("{{{{{key}}}}}"), value);
        }
        out
    }
}
