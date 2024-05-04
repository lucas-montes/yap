use lsp_server::{Connection, ExtractError, Message, Request, RequestId, Response};
use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionList, CompletionParams, HoverContents,
    InitializeParams, LanguageString, MarkedString, ServerCapabilities,
    TextDocumentPositionParams,
};
use std::{collections::HashMap, error::Error};
use tracing::{debug, error};
use tracing_appender::non_blocking;
use tracing_subscriber::{
    fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt,
};
use tree_sitter::{Node, Parser, Point, Query, QueryCursor};

pub fn run_server() -> Result<(), Box<dyn Error + Sync + Send>> {
    // Note that  we must have our logging only write out to stderr.
    // tracing_subscriber::fmt::init();

    let (non_blocking, _guard) = non_blocking(std::io::stderr());
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "lsp=trace".into()),
        )
        .with(
            tracing_subscriber::fmt::layer()
                .json()
                .with_writer(non_blocking)
                .log_internal_errors(true)
                .with_file(true)
                .with_line_number(true)
                .with_thread_ids(true)
                .with_thread_names(true)
                .with_current_span(true)
                .with_span_events(FmtSpan::FULL)
                .with_span_list(true)
                .with_target(true),
        )
        .init();

    // Create the transport. Includes the stdio (stdin and stdout) versions but this could
    // also be implemented to use sockets or HTTP.
    let (connection, io_threads) = Connection::stdio();

    // Run the server and wait for the two threads to end (typically by trigger LSP Exit event).
    let server_capabilities = serde_json::to_value(ServerCapabilities {
        // text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
        hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
        ..Default::default()
    })
    .unwrap();

    let initialization_params = connection.initialize(server_capabilities)?;
    main_loop(connection, initialization_params)?;
    io_threads.join()?;

    // Shut down gracefully.
    error!("shutting down server");
    Ok(())
}
#[allow(dead_code)]
#[derive(serde::Deserialize, Debug)]
struct Text {
    text: String,
}
#[allow(dead_code)]
#[derive(serde::Deserialize, Debug)]
struct TextDocumentLocation {
    uri: String,
}
#[allow(dead_code)]
#[derive(serde::Deserialize, Debug)]
struct TextDocumentChanges {
    #[serde(rename = "textDocument")]
    text_document: TextDocumentLocation,

    #[serde(rename = "contentChanges")]
    content_changes: Vec<Text>,
}
#[allow(dead_code)]
#[derive(serde::Deserialize, Debug)]
struct TextDocumentOpened {
    uri: String,

    text: String,
}
#[allow(dead_code)]
#[derive(serde::Deserialize, Debug)]
struct TextDocumentOpen {
    #[serde(rename = "textDocument")]
    text_document: TextDocumentOpened,
}

#[derive(Debug, Clone)]
pub struct KbCompletion {
    pub name: &'static str,
    pub desc: &'static str,
}

#[derive(Debug)]
pub struct KbWordAttributeCompletion {
    pub items: Vec<KbCompletion>,
    pub id: RequestId,
}

#[derive(Debug)]
pub struct KbWordAttributeHoverResult {
    pub id: RequestId,
    pub value: String,
}

#[derive(Debug)]
pub enum KbWordResult {
    AttributeHover(KbWordAttributeHoverResult),
}
#[derive(Debug, Clone, PartialEq)]
pub enum Position {
    AttributeName(String),
    AttributeValue { name: String, value: String },
}

fn get_position_from_lsp_completion(
    text_params: TextDocumentPositionParams,
) -> Option<Position> {
    let text = "get_text_document(text_params.text_document.uri)?".to_string();

    let pos = text_params.position;

    // TODO: Gallons of perf work can be done starting here
    let mut parser = Parser::new();

    parser
        .set_language(tree_sitter_md::language())
        .expect("could not load html grammer");

    let tree = parser.parse(&text, None)?;
    let root_node = tree.root_node();
    let trigger_point = Point::new(pos.line as usize, pos.character as usize);
    query_position(root_node, text.as_str(), trigger_point)
}

fn query_position(
    root: Node<'_>,
    source: &str,
    trigger_point: Point,
) -> Option<Position> {
    debug!("query_position entering");
    let closest_node = match root.descendant_for_point_range(trigger_point, trigger_point)
    {
        Some(node) => node,
        None => {
            debug!("query_position closest_node not found");
            return None;
        },
    };
    debug!("query_position closest_node {:?}", closest_node);
    let element = match find_element_referent_to_current_node(closest_node) {
        Some(node) => node,
        None => {
            debug!("query_position find_element_referent_to_current_node not found");
            return None;
        },
    };
    debug!("query_position element {:?}", element);
    let attr_completion = query_attr_keys_for_completion(element, source, trigger_point);
    debug!("query_position attr_completion {:?}", attr_completion);
    attr_completion
}
fn query_attr_keys_for_completion(
    node: Node<'_>,
    source: &str,
    trigger_point: Point,
) -> Option<Position> {
    // [ ] means match any of the following
    let query_string = r#"
    (
        [
            (_ 
                (tag_name) 

                (_)*

                (attribute (attribute_name) @attr_name) @complete_match

                (#eq? @attr_name @complete_match)
            )

            (_ 
              (tag_name) 

              (attribute (attribute_name)) 

              (ERROR)
            ) @unfinished_tag
        ]

        (#match? @attr_name "*")
    )"#;

    let props = query_props(query_string, node, source, trigger_point);
    debug!("query_attr_keys_for_completion props {:?}", props);
    let attr_name = match props.get("attr_name") {
        Some(d) => d,
        None => {
            debug!("query_attr_keys_for_completion attr_name not found");
            return None;
        },
    };
    debug!("query_attr_keys_for_completion attr_name {:?}", attr_name);
    if props.get("unfinished_tag").is_some() {
        return None;
    }

    Some(Position::AttributeName(attr_name.value.to_owned()))
}

#[allow(dead_code)]
#[derive(Debug)]
struct CaptureDetails {
    value: String,
    end_position: Point,
}
fn query_props(
    query_string: &str,
    node: Node<'_>,
    source: &str,
    trigger_point: Point,
) -> HashMap<String, CaptureDetails> {
    let query = Query::new(tree_sitter_md::language(), query_string)
        .unwrap_or_else(|_| panic!("get_position_by_query invalid query {query_string}"));
    let mut cursor_qry = QueryCursor::new();

    let capture_names = query.capture_names();

    let matches = cursor_qry.matches(&query, node, source.as_bytes());

    // Only consider the captures that are within the range based on the
    // trigger point (cursor position)
    matches
        .into_iter()
        .flat_map(|m| {
            m.captures
                .iter()
                .filter(|capture| capture.node.start_position() <= trigger_point)
        })
        .fold(HashMap::new(), |mut acc, capture| {
            let key = capture_names[capture.index as usize].to_owned();
            let value =
                if let Ok(capture_value) = capture.node.utf8_text(source.as_bytes()) {
                    capture_value.to_owned()
                } else {
                    error!("query_props capture.node.utf8_text failed {key}");
                    "".to_owned()
                };

            acc.insert(
                key,
                CaptureDetails {
                    value,
                    end_position: capture.node.end_position(),
                },
            );

            acc
        })
}

fn find_element_referent_to_current_node(node: Node<'_>) -> Option<Node<'_>> {
    if node.kind() == "element" || node.kind() == "fragment" {
        debug!(
            "find_element_referent_to_current_node we found something {:?}",
            node
        );
        return Some(node);
    }

    return find_element_referent_to_current_node(node.parent()?);
}

pub fn kb_hover(text_params: TextDocumentPositionParams) -> Option<KbCompletion> {
    let result = get_position_from_lsp_completion(text_params.clone())?;
    debug!("inside kbhover {:?}", result);
    match result {
        Position::AttributeName(name) => KB_TAGS.iter().find(|x| x.name == name).cloned(),
        Position::AttributeValue { name, .. } => {
            KB_TAGS.iter().find(|x| x.name == name).cloned()
        },
    }
}

macro_rules! build_completion {
    ($(($name:expr, $desc:expr)),*) => {
        &[
            $(KbCompletion {
            name: $name,
            desc: include_str!($desc),
            }),*
        ]
    };
}

pub static KB_TAGS: &[KbCompletion] = build_completion!(
    // ("first", "../../test-data/docs/first.md"),
    // ("second", "../../test-data/docs/second.md"),
    // ("third", "../../test-data/docs/third.md")
);

fn handle_hover(req: Request) -> Option<KbWordResult> {
    let completion: CompletionParams = serde_json::from_value(req.params).ok()?;
    let text_params = completion.text_document_position;

    debug!("requesting kbhover {:?}", text_params);
    let attribute = kb_hover(text_params)?;

    debug!("using kbhover {:?}", attribute);
    Some(KbWordResult::AttributeHover(KbWordAttributeHoverResult {
        id: req.id,
        value: attribute.desc.to_string(),
    }))
}

fn handle_request(req: Request) -> Option<KbWordResult> {
    match req.method.as_str() {
        "textDocument/hover" => handle_hover(req),
        _ => {
            error!("unhandled request: {:?}", req);
            None
        },
    }
}

#[allow(dead_code)]
fn to_completion_list(items: Vec<KbCompletion>) -> CompletionList {
    return CompletionList {
        is_incomplete: true,
        items: items
            .iter()
            .map(|x| CompletionItem {
                label: x.name.to_string(),
                label_details: None,
                kind: Some(CompletionItemKind::TEXT),
                detail: Some(x.desc.to_string()),
                documentation: None,
                deprecated: Some(false),
                preselect: None,
                sort_text: None,
                filter_text: None,
                insert_text: None,
                insert_text_format: None,
                insert_text_mode: None,
                text_edit: None,
                additional_text_edits: None,
                command: None,
                commit_characters: None,
                data: None,
                tags: None,
            })
            .collect(),
    };
}
fn main_loop(
    connection: Connection,
    params: serde_json::Value,
) -> Result<(), Box<dyn Error + Sync + Send>> {
    let _params: InitializeParams = serde_json::from_value(params).unwrap();

    for msg in &connection.receiver {
        let result = match msg {
            Message::Request(req) => handle_request(req),
            _ => None,
        };

        match match result {
            Some(KbWordResult::AttributeHover(hover_resp)) => {
                let hover_response = lsp_types::Hover {
                    contents: HoverContents::Scalar(MarkedString::LanguageString(
                        LanguageString {
                            language: "markdown".to_string(),
                            value: hover_resp.value.clone(),
                        },
                    )),
                    range: None,
                };

                let str = match serde_json::to_value(&hover_response) {
                    Ok(s) => s,
                    Err(err) => {
                        error!("Fail to parse hover_response: {:?}", err);
                        return Ok(());
                    },
                };

                connection.sender.send(Message::Response(Response {
                    id: hover_resp.id,
                    result: Some(str),
                    error: None,
                }))
            },
            None => continue,
        } {
            Ok(_) => {},
            Err(e) => error!("failed to send response: {:?}", e),
        };
    }

    Ok(())
}
#[allow(dead_code)]
fn cast<R>(req: Request) -> Result<(RequestId, R::Params), ExtractError<Request>>
where
    R: lsp_types::request::Request,
    R::Params: serde::de::DeserializeOwned,
{
    req.extract(R::METHOD)
}
