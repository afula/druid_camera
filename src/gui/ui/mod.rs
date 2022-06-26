mod playback;

use druid::{
	theme,
	widget::{Align, Axis, CrossAxisAlignment, Flex, SizedBox, Tabs, TabsEdge},
	Color, Data, ExtEventSink, Lens, UnitPoint, Widget, WidgetExt,
};

use crate::gui::{
	data::{video, AppState},
	widgets::{
		theme::{self as CustomTheme, ThemeScope},
	},
};

/// Build the root UI widget.
pub fn root_widget() -> impl Widget<AppState> {
	let layout = Flex::column()
		.cross_axis_alignment(CrossAxisAlignment::Start)
		.with_flex_child(
			video::VideoView::new()
				.expand()
				.lens(AppState::video),
			1.0,
		)
		.with_spacer(CustomTheme::grid(6.0))
		.with_child(playback::panel_widget())
		.background(theme::BACKGROUND_LIGHT);

	let sized = SizedBox::new(layout)
		.width(320.0)
		.height(240.0)
		.expand()
		.border(Color::grey(0.6), 2.0)
		.center()
		.boxed();

	// layout
	ThemeScope::new(Align::centered(sized))
	// ThemeScope::new(layout)
}
