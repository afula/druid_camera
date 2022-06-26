// impl<W: Widget<AppData>> Controller<AppData, W> for VideoController {
//     fn event(
//         &mut self,
//         child: &mut W,
//         ctx: &mut EventCtx,
//         event: &Event,
//         data: &mut AppData,
//         env: &Env,
//     ) {
//         match event {
//             Event::Command(cmd) if cmd.is(EDIT_BEGAN) => {
//                 let widget_id = *cmd.get_unchecked(EDIT_BEGAN);
//                 data.active_message = match widget_id {
//                     DOLLAR_ERROR_WIDGET => Some(DOLLAR_EXPLAINER),
//                     EURO_ERROR_WIDGET => Some(EURO_EXPLAINER),
//                     POUND_ERROR_WIDGET => Some(POUND_EXPLAINER),
//                     POSTAL_ERROR_WIDGET => Some(POSTAL_EXPLAINER),
//                     CAT_ERROR_WIDGET => Some(CAT_EXPLAINER),
//                     _ => unreachable!(),
//                 };
//                 data.active_textbox = Some(widget_id);
//             }
//             Event::Command(cmd) if cmd.is(EDIT_FINISHED) => {
//                 let finished_id = *cmd.get_unchecked(EDIT_FINISHED);
//                 if data.active_textbox == Some(finished_id) {
//                     data.active_textbox = None;
//                     data.active_message = None;
//                 }
//             }
//             _ => child.event(ctx, event, data, env),
//         }
//     }
// }
