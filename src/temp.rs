
// if self.state.selected_radio_widget == Some(Widget::Text) {
//     match self.state.draw_mode {
//         DrawMode::DrawAll => None,
//         DrawMode::Edit => None,
//         DrawMode::New => {
//             match program_state {
//                 Some(Pending::New { 
//                     widget 
//                 }) => {
//                     let update_widget = add_keypress(widget, modified_key);
//                     *program_state = Some(Pending::New { 
//                         widget: update_widget.unwrap(), 
//                     });
//                     update_widget
//                 },
//                 _ => None,
//             }
//         },
//         DrawMode::Rotate => None,
//     }
// }



// Event::Keyboard(key_event) => {
                //     let message = match key_event {
                //         iced::keyboard::Event::KeyPressed { 
                //             key:_, 
                //             modified_key, 
                //             physical_key:_, 
                //             location:_, 
                //             modifiers:_, 
                //             text:_ } => {
                //                 if self.state.selected_radio_widget == Some(Widget::Text) {
                //                     match self.state.draw_mode {
                //                         DrawMode::DrawAll => None,
                //                         DrawMode::Edit => None,
                //                         DrawMode::New => {
                //                             match program_state {
                //                                 Some(Pending::New { 
                //                                     widget 
                //                                 }) => {
                //                                     let update_widget = add_keypress(widget, modified_key);
                //                                     *program_state = Some(Pending::New { 
                //                                         widget: update_widget.unwrap(), 
                //                                     });
                //                                     update_widget
                //                                 },
                //                                 _ => None,
                //                             }
                //                         },
                //                         DrawMode::Rotate => None,
                //                     }
                //                 }
                //             None
                //             },
                //         iced::keyboard::Event::KeyReleased {key: _, location:_, modifiers:_ } => None,
                //         iced::keyboard::Event::ModifiersChanged(_) => None,
                //     };
                // }