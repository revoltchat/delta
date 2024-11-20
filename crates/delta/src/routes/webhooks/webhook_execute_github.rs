use revolt_database::{util::reference::Reference, Database, Message};
use revolt_models::v0::{MessageAuthor, SendableEmbed, Webhook};
use revolt_result::{create_error, Error, Result};
use revolt_rocket_okapi::{
    gen::OpenApiGenerator,
    request::{OpenApiFromRequest, RequestHeaderInput},
    revolt_okapi::openapi3::{MediaType, Parameter, ParameterValue},
};
use rocket::{http::Status, request::FromRequest, Request, State};
use schemars::schema::SchemaObject;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use ulid::Ulid;
use validator::Validate;

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct GithubUser {
    name: Option<String>,
    email: Option<String>,
    login: String,
    id: u32,
    node_id: String,
    avatar_url: String,
    gravatar_id: String,
    url: String,
    html_url: String,
    followers_url: String,
    following_url: String,
    gists_url: String,
    starred_url: String,
    subscriptions_url: String,
    organizations_url: String,
    repos_url: String,
    events_url: String,
    received_events_url: String,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct GithubRepositorySecurityAndAnalysisStatus {
    status: String,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct GithubRepositorySecurityAndAnalysis {
    advanced_security: GithubRepositorySecurityAndAnalysisStatus,
    secret_scanning: GithubRepositorySecurityAndAnalysisStatus,
    secret_scanning_push_protection: GithubRepositorySecurityAndAnalysisStatus,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct GithubRepositoryLicense {
    key: Option<String>,
    name: Option<String>,
    spdx_id: Option<String>,
    url: Option<String>,
    node_id: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct GithubRepositoryCodeOfConduct {
    key: String,
    name: String,
    url: String,
    body: Option<String>,
    html_url: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct GithubRepositoryPermissions {
    admin: Option<bool>,
    maintain: Option<bool>,
    push: Option<bool>,
    triage: Option<bool>,
    pull: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct GithubRepository {
    id: u32,
    node_id: String,
    name: String,
    full_name: String,
    owner: GithubUser,
    private: bool,
    html_url: String,
    description: Option<String>,
    fork: bool,
    url: String,
    archive_url: String,
    assignees_url: String,
    blobs_url: String,
    branches_url: String,
    collaborators_url: String,
    comments_url: String,
    commits_url: String,
    compare_url: String,
    contents_url: String,
    contributors_url: String,
    deployments_url: String,
    downloads_url: String,
    events_url: String,
    forks_url: String,
    git_commits_url: String,
    git_refs_url: String,
    git_tags_url: String,
    git_url: Option<String>,
    issue_comment_url: String,
    issue_events_url: String,
    issues_url: String,
    keys_url: String,
    labels_url: String,
    languages_url: String,
    merges_url: String,
    milestones_url: String,
    notifications_url: String,
    pulls_url: String,
    releases_url: String,
    ssh_url: String,
    stargazers_url: String,
    statuses_url: String,
    subscribers_url: String,
    subscription_url: String,
    tags_url: String,
    teams_url: String,
    trees_url: String,
    clone_url: Option<String>,
    mirror_url: Option<String>,
    hooks_url: String,
    svn_url: Option<String>,
    homepage: Option<String>,
    language: Option<String>,
    forks_count: Option<u32>,
    stargazers_count: Option<u32>,
    watchers_count: Option<u32>,
    size: Option<u64>,
    default_branch: Option<String>,
    open_issues_count: Option<u32>,
    is_template: Option<bool>,
    topics: Option<Vec<String>>,
    has_issues: Option<bool>,
    has_projects: Option<bool>,
    has_wiki: Option<bool>,
    has_pages: Option<bool>,
    has_downloads: Option<bool>,
    has_discussions: Option<bool>,
    archived: Option<bool>,
    disabled: Option<bool>,
    visibility: Option<String>,
    pushed_at: Option<Value>,
    created_at: Value,
    updated_at: Option<Value>,
    permissions: Option<GithubRepositoryPermissions>,
    role_name: Option<String>,
    temp_clone_token: Option<String>,
    delete_branch_on_merge: Option<bool>,
    subscribers_count: Option<u32>,
    network_count: Option<u32>,
    code_of_conduct: Option<GithubRepositoryCodeOfConduct>,
    license: Option<GithubRepositoryLicense>,
    forks: Option<u32>,
    open_issues: Option<u32>,
    watchers: Option<u32>,
    allow_forking: Option<bool>,
    web_commit_signoff_required: Option<bool>,
    security_and_analysis: Option<GithubRepositorySecurityAndAnalysis>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct CommitAuthor {
    date: Option<String>,
    email: Option<String>,
    name: String,
    username: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct GithubCommit {
    added: Option<Vec<String>>,
    author: CommitAuthor,
    committer: CommitAuthor,
    distinct: bool,
    id: String,
    message: String,
    modified: Option<Vec<String>>,
    removed: Option<Vec<String>>,
    timestamp: String,
    tree_id: String,
    url: String,
}
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct GithubReactions {
    #[serde(rename = "+1")]
    plus_one: u32,
    #[serde(rename = "-1")]
    minus_one: u32,
    confused: u32,
    eyes: u32,
    heart: u32,
    hooray: u32,
    laugh: u32,
    rocket: u32,
    total_count: u32,
    url: String,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct GithubComment {
    author_association: String,
    body: String,
    commit_id: Option<String>,
    created_at: Value,
    html_url: String,
    id: u32,
    line: Option<u32>,
    node_id: String,
    path: Option<String>,
    position: Option<u32>,
    reactions: Option<GithubReactions>,
    updated_at: Value,
    url: String,
    user: GithubUser,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct GithubDiscussionComment {
    #[serde(flatten)]
    comment: GithubComment,
    child_comment_count: u32,
    parent_id: Option<u32>,
    repository_url: String,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct GithubDiscussionCategory {
    id: u32,
    node_id: String,
    repository_id: u32,
    emoji: String,
    name: String,
    description: String,
    created_at: Value,
    updated_at: Value,
    slug: String,
    is_answerable: bool,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct GithubDiscussion {
    repository_url: String,
    category: GithubDiscussionCategory,
    answer_html_url: Option<String>,
    answer_chosen_at: Value, // ??
    answer_chosen_by: Value, // ??
    html_url: String,
    id: u32,
    node_id: String,
    number: u32,
    title: String,
    user: GithubUser,
    state: String,
    locked: bool,
    comments: u32,
    created_at: Value,
    updated_at: Value,
    author_association: String,
    active_lock_reason: Option<String>,
    body: String,
    reactions: GithubReactions,
    timeline_url: String,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "action")]
#[allow(clippy::large_enum_variant)]
pub enum GithubDiscussionEvent {
    Created {
        discussion: GithubDiscussion,
    },
    Answered {
        discussion: GithubDiscussion,
        answer: GithubDiscussionComment,
    },
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "action")]
#[allow(clippy::large_enum_variant)]
pub enum DiscussionCommentEvent {
    Created {
        comment: GithubDiscussionComment,
        discussion: GithubDiscussion,
    },
    Deleted {},
    Edited {},
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct GithubMilestone {
    url: String,
    html_url: String,
    labels_url: String,
    id: u32,
    node_id: String,
    number: u32,
    state: String,
    title: String,
    description: String,
    creator: Option<GithubUser>,
    open_issues: u32,
    closed_issues: u32,
    created_at: Value,
    updated_at: Value,
    closed_at: Option<Value>,
    due_on: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct GithubLinkedPullRequest {
    merged_at: Option<Value>,
    diff_url: Option<String>,
    html_url: Option<String>,
    patch_url: Option<String>,
    url: Option<String>,
}
#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct GithubAppPermissions {
    issues: Option<String>,
    checks: Option<String>,
    metadata: Option<String>,
    contents: Option<String>,
    deployments: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct GithubApp {
    id: u32,
    slug: String,
    node_is: String,
    owner: Option<GithubUser>,
    name: String,
    description: Option<String>,
    external_url: String,
    html_url: String,
    created_at: Value,
    updated_at: Value,
    permissions: GithubAppPermissions,
    events: Vec<String>,
    installations_count: Option<u32>,
    client_id: Option<String>,
    client_secret: Option<String>,
    webhook_secret: Option<String>,
    pem: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
pub struct GithubIssue {
    id: u32,
    node_id: String,
    url: String,
    repository_url: String,
    labels_url: String,
    comments_url: String,
    events_url: String,
    html_url: String,
    number: u32,
    state: String,
    state_reason: Option<String>,
    user: Option<GithubUser>,
    labels: Vec<String>,
    assignee: Option<GithubUser>,
    assignees: Vec<GithubUser>,
    milestone: Option<GithubMilestone>,
    locked: bool,
    active_lock_reason: Option<String>,
    comments: u32,
    pull_request: Option<GithubLinkedPullRequest>,
    closed_at: Option<Value>,
    created_at: Value,
    updated_at: Value,
    draft: Option<bool>,
    closed_by: Option<GithubUser>,
    body: Option<String>,
    timeline_url: Option<String>,
    repository: Option<GithubRepository>,
    performed_via_github_app: Option<GithubApp>,
    author_association: String,
    reactions: Option<GithubReactions>,
    title: String,
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "action")]
#[allow(clippy::large_enum_variant)]
pub enum IssueCommentEvent {
    Created {
        comment: GithubComment,
        issue: GithubIssue,
    },
    Deleted {},
    Edited {},
}

#[derive(Serialize, Deserialize, Debug, JsonSchema)]
#[serde(rename_all = "snake_case", tag = "action")]
#[allow(clippy::large_enum_variant)]
pub enum IssuesEvent {
    Assigned {},
    Closed { issue: GithubIssue },
    Deleted {},
    Demilestoned {},
    Edited {},
    Labeled {},
    Locked {},
    Milestoned {},
    Opened { issue: GithubIssue },
    Pinned {},
    Reopened { issue: GithubIssue },
    Transferred {},
    Unassigned {},
    Unlabeled {},
    Unlocked {},
    Unpinned {},
}

#[derive(Serialize, Deserialize, Debug)]
pub struct StarEvent {
    starred_at: Option<Value>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PushEvent {
    after: String,
    base_ref: Option<String>,
    before: String,
    commits: Vec<GithubCommit>,
    compare: String,
    created: bool,
    deleted: bool,
    forced: bool,
    head_commit: Option<GithubCommit>,
    pusher: CommitAuthor,
    r#ref: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CommitCommentEvent {
    comment: GithubComment,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct CreateEvent {
    master_branch: String,
    pusher_type: String,
    r#ref: String,
    ref_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct DeleteEvent {
    pusher_type: String,
    r#ref: String,
    ref_type: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ForkEvent {
    forkee: GithubRepository,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GithubTeam {
    id: u32,
    node_id: String,
    url: String,
    hmtl_url: String,
    name: String,
    slug: String,
    description: Option<String>,
    privacy: String,
    permission: String,
    members_url: String,
    repositories_url: String,
    parent: Option<Box<GithubTeam>>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GithubHead {
    label: String,
    r#ref: String,
    repo: GithubRepository,
    sha: String,
    user: Option<GithubUser>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GithubHref {
    href: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GithubLinks {
    #[serde(rename = "self")]
    _self: GithubHref,
    html: GithubHref,
    comments: GithubHref,
    commits: GithubHref,
    statuses: GithubHref,
    issue: GithubHref,
    review_comments: GithubHref,
    review_comment: GithubHref,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GithubAutoMerge {
    enabled_by: GithubUser,
    merge_method: String,
    commit_title: String,
    commit_message: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct GithubPullRequest {
    url: String,
    id: u32,
    node_id: String,
    html_url: String,
    diff_url: String,
    patch_url: String,
    issue_url: String,
    number: u32,
    state: String,
    locked: bool,
    title: String,
    user: GithubUser,
    body: Option<String>,
    created_at: Value,
    updated_at: Value,
    closed_at: Option<Value>,
    merged_at: Option<Value>,
    merge_commit_sha: Option<String>,
    assignee: Option<GithubUser>,
    assignees: Vec<GithubUser>,
    requested_reviewers: Vec<GithubUser>,
    requested_teams: Vec<GithubTeam>,
    head: GithubHead,
    base: GithubHead,
    _links: GithubLinks,
    author_association: String,
    auto_merge: Option<GithubAutoMerge>,
    draft: bool,
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case", tag = "action")]
pub enum PullRequestEvent {
    Assigned {},
    AutoMergeDisabled {},
    AutoMergeEnabled {},
    Closed {
        number: u32,
        pull_request: GithubPullRequest,
    },
    ConvertedToDraft {},
    Demilestoned {},
    Dequeued {},
    Edited {},
    Enqueued {},
    Labeled {},
    Locked {},
    Milestoned {},
    Opened {
        number: u32,
        pull_request: GithubPullRequest,
    },
    ReadyForReview {},
    Reopened {
        number: u32,
        pull_request: GithubPullRequest,
    },
    ReviewRequestRemoved {},
    ReviewRequest {},
    Synchronized {},
    Unassigned {},
    Unlabeled {},
    Unlocked {},
}

#[derive(Debug)]
#[allow(clippy::large_enum_variant)]
pub enum BaseEvent {
    Star(StarEvent),
    Ping,
    Push(PushEvent),
    CommitComment(CommitCommentEvent),
    Create(CreateEvent),
    Delete(DeleteEvent),
    Discussion(GithubDiscussionEvent),
    DiscussionComment(DiscussionCommentEvent),
    Fork(ForkEvent),
    IssueComment(IssueCommentEvent),
    Issues(IssuesEvent),
    PullRequest(PullRequestEvent),
}

#[derive(Serialize, Deserialize, Debug)]
pub struct _Event {
    action: Option<String>,
    sender: GithubUser,
    repository: GithubRepository,
}

#[derive(Debug)]
pub struct Event {
    event: BaseEvent,
    action: Option<String>,
    sender: GithubUser,
    repository: GithubRepository,
}

#[derive(Debug, JsonSchema)]
pub struct EventHeader<'r>(pub &'r str);

impl<'r> std::ops::Deref for EventHeader<'r> {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        self.0
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for EventHeader<'r> {
    type Error = Error;

    async fn from_request(request: &'r Request<'_>) -> rocket::request::Outcome<Self, Self::Error> {
        let headers = request.headers();
        let Some(event) = headers.get_one("X-GitHub-Event") else {
            return rocket::request::Outcome::Error((
                Status::BadRequest,
                create_error!(InvalidOperation),
            ));
        };

        rocket::request::Outcome::Success(Self(event))
    }
}

impl<'r> OpenApiFromRequest<'r> for EventHeader<'r> {
    fn from_request_input(
        _gen: &mut OpenApiGenerator,
        _name: String,
        _required: bool,
    ) -> revolt_rocket_okapi::Result<RequestHeaderInput> {
        let mut content = schemars::Map::new();
        content.insert(
            "X-Github-Event".to_string(),
            MediaType {
                schema: Some(SchemaObject {
                    string: Some(Box::default()),
                    ..Default::default()
                }),
                example: None,
                examples: None,
                encoding: schemars::Map::new(),
                extensions: schemars::Map::new(),
            },
        );

        Ok(RequestHeaderInput::Parameter(Parameter {
            name: "X-Github-Event".to_string(),
            location: "header".to_string(),
            required: true,
            description: Some("The name of the github event".to_string()),
            deprecated: false,
            allow_empty_value: false,
            value: ParameterValue::Content { content },
            extensions: schemars::Map::new(),
        }))
    }
}

const GREEN: &str = "#0f890f";
const RED: &str = "#e73e3e";
const GREY: &str = "#202224";
const BLUE: &str = "#279adc";
const ORANGE: &str = "#d76a34";
const LIGHT_ORANGE: &str = "#d9916d";
// for future use
// const WHITE: &str = "#c3e1c3";

fn shorten_text(text: &str, length: usize) -> String {
    if text.len() < length {
        text.to_string()
    } else {
        format!("{}...", &text[0..length])
    }
}

fn safe_from_str<T: for<'de> Deserialize<'de>>(data: &str) -> Result<T> {
    match serde_json::from_str(data) {
        Ok(output) => Ok(output),
        Err(err) => {
            log::error!("{err:?}");
            Err(create_error!(InvalidOperation))
        }
    }
}

// Because github gives the event name in the header, this requires us to manually parse the enum, i did try to manually edit the json to add the tag into it,
// however it seems serde messes up with #[serde(flattern)] and enum handling.
// If someone finds a better solution to this please make a PR.

fn convert_event(data: &str, event_name: &str) -> Result<Event> {
    let event = safe_from_str(data)?;

    let base_event = match event_name {
        "star" => BaseEvent::Star(safe_from_str(data)?),
        "ping" => BaseEvent::Ping,
        "push" => BaseEvent::Push(safe_from_str(data)?),
        "commit_comment" => BaseEvent::CommitComment(safe_from_str(data)?),
        "create" => BaseEvent::Create(safe_from_str(data)?),
        "delete" => BaseEvent::Delete(safe_from_str(data)?),
        "discussion" => BaseEvent::Discussion(safe_from_str(data)?),
        "discussion_comment" => BaseEvent::DiscussionComment(safe_from_str(data)?),
        "fork" => BaseEvent::Fork(safe_from_str(data)?),
        "issue_comment" => BaseEvent::IssueComment(safe_from_str(data)?),
        "issues" => BaseEvent::Issues(safe_from_str(data)?),
        "pull_request" => BaseEvent::PullRequest(safe_from_str(data)?),
        _ => return Err(create_error!(InvalidOperation)),
    };

    let _Event {
        action,
        sender,
        repository,
    } = event;

    Ok(Event {
        action,
        sender,
        repository,
        event: base_event,
    })
}

/// # Executes a webhook specific to github
///
/// Executes a webhook specific to github and sends a message containing the relevant info about the event
#[openapi(tag = "Webhooks")]
#[post("/<webhook_id>/<token>/github", data = "<data>")]
pub async fn webhook_execute_github(
    db: &State<Database>,
    webhook_id: Reference,
    token: String,
    event: EventHeader<'_>,
    data: String,
) -> Result<()> {
    let webhook = webhook_id.as_webhook(db).await?;
    webhook.assert_token(&token)?;

    let channel = db.fetch_channel(&webhook.channel_id).await?;
    let event = convert_event(&data, &event)?;

    let sendable_embed = match event.event {
        BaseEvent::Star(_) => {
            if event.action.as_deref() != Some("created") {
                return Ok(());
            };

            SendableEmbed {
                title: Some(event.sender.login),
                description: Some(format!(
                    "#### [[{}] New star added]({})",
                    event.repository.full_name, event.repository.html_url
                )),
                colour: Some(GREY.to_string()),
                icon_url: Some(event.sender.avatar_url),
                url: Some(event.sender.html_url),
                ..Default::default()
            }
        }
        BaseEvent::Push(PushEvent {
            after,
            commits,
            compare,
            forced,
            r#ref,
            ..
        }) => {
            let Some(branch) = r#ref.split('/').nth(2) else {
                return Ok(());
            };

            if forced {
                let description = format!(
                    "#### [{}] Branch {} was force-pushed to {}\n[compare changes]({})",
                    event.repository.full_name,
                    branch,
                    &after[0..=7],
                    compare
                );

                SendableEmbed {
                    icon_url: Some(event.sender.avatar_url),
                    title: Some(event.sender.login),
                    description: Some(description),
                    url: Some(event.sender.html_url),
                    colour: Some(RED.to_string()),
                    ..Default::default()
                }
            } else {
                let title = format!(
                    "[[{}:{}] {} new commit]({})",
                    event.repository.full_name,
                    branch,
                    commits.len(),
                    compare
                );
                let commit_description = commits
                    .into_iter()
                    .map(|commit| {
                        format!(
                            "[`{}`]({}) {} - {}",
                            &commit.id[0..=7],
                            commit.url,
                            shorten_text(&commit.message, 50),
                            commit.author.name
                        )
                    })
                    .collect::<Vec<String>>()
                    .join("\n");

                SendableEmbed {
                    title: Some(event.sender.login),
                    description: Some(format!("#### {title}\n{commit_description}")),
                    colour: Some(BLUE.to_string()),
                    icon_url: Some(event.sender.avatar_url),
                    url: Some(event.sender.html_url),
                    ..Default::default()
                }
            }
        }
        BaseEvent::CommitComment(CommitCommentEvent { comment }) => {
            let commit_id = match comment.commit_id {
                Some(id) => id[0..=7].to_string(),
                None => "".to_string(),
            };

            SendableEmbed {
                icon_url: Some(event.sender.avatar_url),
                url: Some(event.sender.html_url),
                title: Some(event.sender.login),
                description: Some(format!(
                    "#### [[{}] New comment on commit `{}`]({})\n{}",
                    event.repository.full_name,
                    commit_id,
                    comment.html_url,
                    shorten_text(&comment.body, 450)
                )),
                colour: Some(GREY.to_string()),
                ..Default::default()
            }
        }
        BaseEvent::Ping => return Ok(()),
        BaseEvent::Create(CreateEvent {
            r#ref, ref_type, ..
        }) => SendableEmbed {
            icon_url: Some(event.sender.avatar_url),
            url: Some(event.sender.html_url),
            title: Some(event.sender.login),
            description: Some(format!(
                "#### [{}] New {} created: {}",
                event.repository.full_name, ref_type, r#ref
            )),
            colour: Some(GREY.to_string()),
            ..Default::default()
        },
        BaseEvent::Delete(DeleteEvent {
            r#ref, ref_type, ..
        }) => SendableEmbed {
            icon_url: Some(event.sender.avatar_url),
            url: Some(event.sender.html_url),
            title: Some(event.sender.login),
            description: Some(format!(
                "#### [{}] {} deleted: {}",
                event.repository.full_name, ref_type, r#ref
            )),
            colour: Some(GREY.to_string()),
            ..Default::default()
        },
        BaseEvent::Discussion(discussion_event) => match discussion_event {
            GithubDiscussionEvent::Created { discussion } => SendableEmbed {
                icon_url: Some(event.sender.avatar_url),
                url: Some(event.sender.html_url),
                title: Some(event.sender.login),
                description: Some(format!(
                    "#### [{}] New discussion #{}: {}\n{}",
                    event.repository.full_name,
                    discussion.number,
                    discussion.title,
                    shorten_text(&discussion.body, 450)
                )),
                colour: Some(LIGHT_ORANGE.to_string()),
                ..Default::default()
            },
            GithubDiscussionEvent::Answered { discussion, answer } => SendableEmbed {
                icon_url: Some(answer.comment.user.avatar_url),
                url: Some(answer.comment.user.html_url),
                title: Some(answer.comment.user.login),
                description: Some(format!(
                    "#### [{}] discussion #{} marked answered: {}\n{}",
                    event.repository.full_name,
                    discussion.number,
                    discussion.title,
                    shorten_text(&answer.comment.body, 450)
                )),
                colour: Some(LIGHT_ORANGE.to_string()),
                ..Default::default()
            },
        },
        BaseEvent::DiscussionComment(comment_event) => match comment_event {
            DiscussionCommentEvent::Created {
                comment,
                discussion,
            } => SendableEmbed {
                icon_url: Some(comment.comment.user.avatar_url),
                url: Some(comment.comment.user.html_url),
                title: Some(comment.comment.user.login),
                description: Some(format!(
                    "[{}] New comment on discussion #{}: {}\n{}",
                    event.repository.full_name,
                    discussion.number,
                    discussion.title,
                    shorten_text(&comment.comment.body, 450)
                )),
                colour: Some(LIGHT_ORANGE.to_string()),
                ..Default::default()
            },
            _ => return Ok(()),
        },
        BaseEvent::Fork(ForkEvent { forkee }) => SendableEmbed {
            icon_url: Some(event.sender.avatar_url),
            url: Some(event.sender.html_url),
            title: Some(event.sender.login),
            description: Some(format!(
                "#### [[{}] Fork created: {}]({})",
                event.repository.full_name, forkee.full_name, forkee.html_url
            )),
            colour: Some(GREY.to_string()),
            ..Default::default()
        },
        BaseEvent::Issues(issue_event) => match issue_event {
            IssuesEvent::Closed { issue } => SendableEmbed {
                icon_url: Some(event.sender.avatar_url),
                url: Some(event.sender.html_url),
                title: Some(event.sender.login),
                description: Some(format!(
                    "#### [[{}] Issue Closed #{}: {}]({})",
                    event.repository.full_name, issue.number, issue.title, issue.html_url
                )),
                colour: Some(GREY.to_string()),
                ..Default::default()
            },
            IssuesEvent::Opened { issue } => SendableEmbed {
                icon_url: Some(event.sender.avatar_url),
                url: Some(event.sender.html_url),
                title: Some(event.sender.login),
                description: Some(format!(
                    "#### [[{}] Issue Opened #{}: {}]({})\n{}",
                    event.repository.full_name,
                    issue.number,
                    issue.title,
                    issue.html_url,
                    shorten_text(&issue.body.unwrap_or_default(), 450)
                )),
                colour: Some(ORANGE.to_string()),
                ..Default::default()
            },
            IssuesEvent::Reopened { issue } => SendableEmbed {
                icon_url: Some(event.sender.avatar_url),
                url: Some(event.sender.html_url),
                title: Some(event.sender.login),
                description: Some(format!(
                    "#### [[{}] Issue Reopened #{}: {}]({})",
                    event.repository.full_name, issue.number, issue.html_url, issue.title
                )),
                colour: Some(GREEN.to_string()),
                ..Default::default()
            },
            _ => return Ok(()),
        },
        BaseEvent::IssueComment(comment_event) => match comment_event {
            IssueCommentEvent::Created { comment, issue } => SendableEmbed {
                icon_url: Some(event.sender.avatar_url),
                url: Some(event.sender.html_url),
                title: Some(event.sender.login),
                description: Some(format!(
                    "#### [[{}] New comment on issue #{}: {}]({})\n{}",
                    event.repository.full_name,
                    issue.number,
                    issue.title,
                    issue.html_url,
                    shorten_text(&comment.body, 450)
                )),
                colour: Some(LIGHT_ORANGE.to_string()),
                ..Default::default()
            },
            _ => return Ok(()),
        },
        BaseEvent::PullRequest(pull_request_event) => match pull_request_event {
            PullRequestEvent::Closed {
                number,
                pull_request,
            } => SendableEmbed {
                icon_url: Some(event.sender.avatar_url),
                url: Some(event.sender.html_url),
                title: Some(event.sender.login),
                description: Some(format!(
                    "#### [[{}] Pull Request Closed #{}: {}]({})",
                    event.repository.full_name, number, pull_request.title, pull_request.html_url
                )),
                colour: Some(GREY.to_string()),
                ..Default::default()
            },
            PullRequestEvent::Opened {
                number,
                pull_request,
            } => SendableEmbed {
                icon_url: Some(event.sender.avatar_url),
                url: Some(event.sender.html_url),
                title: Some(event.sender.login),
                description: Some(format!(
                    "#### [[{}] Pull Request Opened #{}: {}]({})\n{}",
                    event.repository.full_name,
                    number,
                    pull_request.title,
                    pull_request.html_url,
                    shorten_text(&pull_request.body.unwrap_or_default(), 450)
                )),
                colour: Some(ORANGE.to_string()),
                ..Default::default()
            },
            PullRequestEvent::Reopened {
                number,
                pull_request,
            } => SendableEmbed {
                icon_url: Some(event.sender.avatar_url),
                url: Some(event.sender.html_url),
                title: Some(event.sender.login),
                description: Some(format!(
                    "#### [[{}] Pull Request Reopened #{}: {}]({})",
                    event.repository.full_name, number, pull_request.html_url, pull_request.title
                )),
                colour: Some(GREEN.to_string()),
                ..Default::default()
            },
            _ => return Ok(()),
        },
    };

    sendable_embed.validate().map_err(|error| {
        create_error!(FailedValidation {
            error: error.to_string()
        })
    })?;

    let message_id = Ulid::new().to_string();

    let mut message = Message {
        id: message_id,
        author: webhook.id.clone(),
        channel: webhook.channel_id.clone(),
        webhook: Some(std::convert::Into::<Webhook>::into(webhook.clone()).into()),
        ..Default::default()
    };

    #[allow(clippy::disallowed_methods)]
    message.attach_sendable_embed(db, sendable_embed).await?;
    message
        .send(
            db,
            MessageAuthor::Webhook(&webhook.into()),
            None,
            None,
            &channel,
            false,
        )
        .await
}
