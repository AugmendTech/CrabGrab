use std::rc::Rc;

use ashpd::{
    desktop::{
        screencast::{CursorMode, Screencast, SourceType},
        Session,
    },
    enumflags2::BitFlags,
};

use crate::{
    capturable_content::{CapturableContentError, CapturableContentFilter},
    prelude::Rect,
};

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct WaylandCapturableWindow {
    pub pw_node_id: u32,
    pub position: Option<(i32, i32)>,
    pub size: Option<(i32, i32)>,
    pub id: Option<String>,
    pub mapping_id: Option<String>,
    pub virt: bool,
    pub cursor_as_metadata: bool,
}

impl WaylandCapturableWindow {
    pub fn from_impl(window: Self) -> Self {
        window
    }

    pub fn title(&self) -> String {
        String::from("n/a")
    }

    pub fn rect(&self) -> Rect {
        let origin = self.position.unwrap_or((0, 0));
        let size = self.size.unwrap_or((0, 0));
        Rect {
            origin: crate::prelude::Point {
                x: origin.0 as f64,
                y: origin.1 as f64,
            },
            size: crate::prelude::Size {
                width: size.0 as f64,
                height: size.1 as f64,
            },
        }
    }

    pub fn application(&self) -> WaylandCapturableApplication {
        WaylandCapturableApplication(())
    }

    pub fn is_visible(&self) -> bool {
        true
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct WaylandCapturableDisplay {
    pub pw_node_id: u32,
    pub position: Option<(i32, i32)>,
    pub size: Option<(i32, i32)>,
    pub id: Option<String>,
    pub mapping_id: Option<String>,
    pub cursor_as_metadata: bool,
}

impl WaylandCapturableDisplay {
    pub fn from_impl(window: Self) -> Self {
        window
    }

    pub fn rect(&self) -> Rect {
        let origin = self.position.unwrap_or((0, 0));
        let size = self.size.unwrap_or((0, 0));
        Rect {
            origin: crate::prelude::Point {
                x: origin.0 as f64,
                y: origin.1 as f64,
            },
            size: crate::prelude::Size {
                width: size.0 as f64,
                height: size.1 as f64,
            },
        }
    }
}

pub struct WaylandCapturableApplication(());

impl WaylandCapturableApplication {
    pub fn identifier(&self) -> String {
        String::from("n/a")
    }

    pub fn name(&self) -> String {
        String::from("n/a")
    }

    pub fn pid(&self) -> i32 {
        -1
    }
}

pub struct WaylandCapturableContent {
    pub windows: Vec<WaylandCapturableWindow>,
    pub displays: Vec<WaylandCapturableDisplay>,
    _sc: Rc<Screencast<'static>>,
    _sc_session: Rc<Session<'static, Screencast<'static>>>,
}

impl WaylandCapturableContent {
    fn source_types_filter(filter: CapturableContentFilter) -> BitFlags<SourceType> {
        let mut bitflags = BitFlags::empty();
        if filter.displays {
            bitflags |= SourceType::Monitor | SourceType::Virtual;
        }
        if let Some(windows_filter) = filter.windows {
            if windows_filter.desktop_windows || windows_filter.onscreen_only {
                bitflags |= SourceType::Window;
            }
        }
        bitflags
    }

    pub async fn new(filter: CapturableContentFilter) -> Result<Self, CapturableContentError> {
        let screencast = Screencast::new()
            .await
            .map_err(|e| CapturableContentError::Other(e.to_string()))?;

        // TODO: Fix dual cursor in kwin when capture monitor and cursor as metadata
        // let cursor_as_metadata = screencast
        //     .available_cursor_modes()
        //     .await
        //     .map_err(|e| CapturableContentError::Other(e.to_string()))?
        //     .contains(CursorMode::Metadata);
        let cursor_as_metadata = false;

        let source_types = Self::source_types_filter(filter)
            // Some portal implementations freak out when we include supported an not supported source types
            & screencast.available_source_types().await.map_err(|e| CapturableContentError::Other(e.to_string()))?;

        if source_types.is_empty() {
            return Err(CapturableContentError::Other(
                "Unsupported content filter".to_string(),
            ));
        }

        let session = screencast
            .create_session()
            .await
            .map_err(|e| CapturableContentError::Other(e.to_string()))?;

        // INVESTIGATE: Show cursor as default when metadata-mode is not available?
        let cursor_mode = if cursor_as_metadata {
            CursorMode::Metadata
        } else {
            CursorMode::Embedded
        };

        screencast
            .select_sources(
                &session,
                cursor_mode,
                source_types,
                false,
                None,
                ashpd::desktop::PersistMode::DoNot,
            )
            .await
            .map_err(|e| CapturableContentError::Other(e.to_string()))?
            .response()
            .map_err(|e| CapturableContentError::Other(e.to_string()))?;
        let streams = screencast
            .start(&session, &ashpd::WindowIdentifier::None)
            .await
            .map_err(|e| CapturableContentError::Other(e.to_string()))?
            .response()
            .map_err(|e| CapturableContentError::Other(e.to_string()))?;

        let mut sources = Self {
            windows: Vec::new(),
            displays: Vec::new(),
            _sc: Rc::new(screencast),
            _sc_session: Rc::new(session),
        };
        for stream in streams.streams() {
            if let Some(source_type) = stream.source_type() {
                match source_type {
                    SourceType::Window | SourceType::Virtual => {
                        sources.windows.push(WaylandCapturableWindow {
                            pw_node_id: stream.pipe_wire_node_id(),
                            position: stream.position(),
                            size: stream.size(),
                            id: stream.id().map(|id| id.to_string()),
                            mapping_id: stream.mapping_id().map(|id| id.to_string()),
                            virt: source_type == SourceType::Virtual,
                            cursor_as_metadata,
                        });
                        continue;
                    }
                    SourceType::Monitor => {}
                }
            }
            // If the stream source_type is `None`, then treat it as a display
            sources.displays.push(WaylandCapturableDisplay {
                pw_node_id: stream.pipe_wire_node_id(),
                position: stream.position(),
                size: stream.size(),
                id: stream.id().map(|id| id.to_string()),
                mapping_id: stream.mapping_id().map(|id| id.to_string()),
                cursor_as_metadata,
            });
        }

        Ok(sources)
    }
}

#[derive(Clone, Default)]
pub(crate) struct WaylandCapturableContentFilter(());

impl WaylandCapturableContentFilter {
    pub(crate) const DEFAULT: Self = Self(());
    pub(crate) const NORMAL_WINDOWS: Self = Self(());
}

#[cfg(test)]
mod tests {
    use ashpd::{desktop::screencast::SourceType, enumflags2::BitFlags};

    use crate::{
        platform::platform_impl::{
            capture_content::WaylandCapturableContent, ImplCapturableContentFilter,
        },
        prelude::{CapturableContentFilter, CapturableWindowFilter},
    };

    #[test]
    fn source_type_filter_conversion_displays() {
        assert_eq!(
            WaylandCapturableContent::source_types_filter(CapturableContentFilter {
                windows: None,
                displays: true,
                impl_capturable_content_filter: ImplCapturableContentFilter::default(),
            }),
            SourceType::Monitor | SourceType::Virtual
        );
    }

    #[test]
    fn source_type_filter_conversion_windows() {
        assert_eq!(
            WaylandCapturableContent::source_types_filter(CapturableContentFilter {
                windows: Some(CapturableWindowFilter {
                    desktop_windows: true,
                    onscreen_only: false
                }),
                displays: false,
                impl_capturable_content_filter: ImplCapturableContentFilter::default(),
            }),
            SourceType::Window
        );
        assert_eq!(
            WaylandCapturableContent::source_types_filter(CapturableContentFilter {
                windows: Some(CapturableWindowFilter {
                    desktop_windows: false,
                    onscreen_only: true
                }),
                displays: false,
                impl_capturable_content_filter: ImplCapturableContentFilter::default(),
            }),
            SourceType::Window
        );
        assert_eq!(
            WaylandCapturableContent::source_types_filter(CapturableContentFilter {
                windows: Some(CapturableWindowFilter {
                    desktop_windows: true,
                    onscreen_only: true
                }),
                displays: false,
                impl_capturable_content_filter: ImplCapturableContentFilter::default(),
            }),
            SourceType::Window
        );
    }

    #[test]
    fn source_type_filter_conversion_none() {
        assert_eq!(
            WaylandCapturableContent::source_types_filter(CapturableContentFilter {
                windows: None,
                displays: false,
                impl_capturable_content_filter: ImplCapturableContentFilter::default(),
            }),
            BitFlags::empty()
        );
    }

    #[test]
    fn source_type_filter_conversion_all() {
        assert_eq!(
            WaylandCapturableContent::source_types_filter(CapturableContentFilter {
                windows: Some(CapturableWindowFilter {
                    desktop_windows: true,
                    onscreen_only: true
                }),
                displays: true,
                impl_capturable_content_filter: ImplCapturableContentFilter::default(),
            }),
            SourceType::Monitor | SourceType::Virtual | SourceType::Window
        );
    }
}
