use std::sync::Arc;

use crate::domain::activity_event::{ActivityCategory, ActivityEvent, ActivityLevel};
use crate::domain::errors::{DomainError, DomainResult};
use crate::domain::ports::repositories::{ActivityRepository, WorkspaceRepository};
use crate::domain::workspace::Workspace;

/// Orchestrates workspace creation/lookup for the onboarding flow (section
/// 38) and the rest of the app, which currently assumes exactly one
/// "current" workspace.
pub struct WorkspaceService {
    workspace_repo: Arc<dyn WorkspaceRepository>,
    activity_repo: Arc<dyn ActivityRepository>,
}

impl WorkspaceService {
    pub fn new(
        workspace_repo: Arc<dyn WorkspaceRepository>,
        activity_repo: Arc<dyn ActivityRepository>,
    ) -> Self {
        Self {
            workspace_repo,
            activity_repo,
        }
    }

    pub async fn get_current(&self) -> DomainResult<Option<Workspace>> {
        self.workspace_repo.get_current().await
    }

    pub async fn create_workspace(&self, name: String) -> DomainResult<Workspace> {
        let trimmed = name.trim();
        if trimmed.is_empty() {
            return Err(DomainError::Validation(
                "workspace name cannot be empty".into(),
            ));
        }
        if trimmed.len() > 80 {
            return Err(DomainError::Validation(
                "workspace name must be 80 characters or fewer".into(),
            ));
        }

        let workspace = Workspace::new(trimmed);
        self.workspace_repo.create(&workspace).await?;

        let event = ActivityEvent::new(
            ActivityCategory::System,
            ActivityLevel::Success,
            format!("Workspace \"{}\" created", workspace.name),
        );
        self.activity_repo.record(&event).await?;

        Ok(workspace)
    }
}
