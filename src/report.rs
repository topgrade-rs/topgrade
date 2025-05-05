use std::borrow::Cow;
use std::fmt::Display;

pub enum StepResult {
    Success(Option<UpdatedComponents>),
    Failure,
    Ignored,
    Skipped(String),
}

impl StepResult {
    pub fn failed(&self) -> bool {
        match self {
            StepResult::Success(_) | StepResult::Ignored | StepResult::Skipped(_) => false,
            StepResult::Failure => true,
        }
    }
}

pub struct UpdatedComponents(Vec<UpdatedComponent>);

impl UpdatedComponents {
    pub fn new(updated: Vec<UpdatedComponent>) -> Self {
        Self(updated)
    }
}

impl Display for UpdatedComponents {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0.as_slice() {
            [] => write!(f, "No updates found"),
            components => {
                writeln!(f, "Updated:")?;
                let updates = components
                    .iter()
                    .map(|c| format!("- {c}"))
                    .collect::<Vec<_>>()
                    .join("\n");
                write!(f, "{}", updates)?;
                Ok(())
            }
        }
    }
}

pub struct UpdatedComponent {
    name: String,
    from_version: Option<String>,
    to_version: Option<String>,
}

impl UpdatedComponent {
    pub fn new(name: String, from_version: Option<String>, to_version: Option<String>) -> Self {
        Self {
            name,
            from_version,
            to_version,
        }
    }
}

impl Display for UpdatedComponent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match (&self.from_version, &self.to_version) {
            (None, None) => write!(f, "{}", self.name),
            (None, Some(to_version)) => write!(f, "{} to {}", self.name, to_version),
            (Some(from_version), None) => write!(f, "{} from {}", self.name, from_version),
            (Some(from_version), Some(to_version)) => {
                write!(f, "{} from {} to {}", self.name, from_version, to_version)
            }
        }
    }
}

type CowString<'a> = Cow<'a, str>;
type ReportData<'a> = Vec<(CowString<'a>, StepResult)>;
pub struct Report<'a> {
    data: ReportData<'a>,
}

impl<'a> Report<'a> {
    pub fn new() -> Self {
        Self { data: Vec::new() }
    }

    pub fn push_result<M>(&mut self, result: Option<(M, StepResult)>)
    where
        M: Into<CowString<'a>>,
    {
        if let Some((key, success)) = result {
            let key = key.into();

            debug_assert!(!self.data.iter().any(|(k, _)| k == &key), "{key} already reported");
            self.data.push((key, success));
        }
    }

    pub fn data(&self) -> &ReportData<'a> {
        &self.data
    }
}
