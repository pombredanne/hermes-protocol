use ffi_utils::RawPointerConverter;
use hermes::HermesProtocolHandler;

#[repr(C)]
#[derive(Debug)]
pub struct CProtocolHandler {
    // hides a Box<HermesProtocolHandler>, note the 2 levels (raw pointer + box) to be sure we have
    // a thin pointer here
    pub handler: *const libc::c_void,
    pub user_data: *mut libc::c_void,
}

pub struct UserData(pub *mut libc::c_void);
unsafe impl Send for UserData {}
unsafe impl Sync for UserData {}

impl UserData {
    pub fn duplicate(&self) -> UserData {
        UserData(self.0)
    }
}

impl CProtocolHandler {
    pub fn new(handler: Box<HermesProtocolHandler>, user_data: *mut libc::c_void) -> Self {
        let user_data = UserData(user_data).into_raw_pointer() as _;
        Self {
            handler: Box::into_raw(Box::new(handler)) as *const libc::c_void,
            user_data,
        }
    }

    pub fn extract(&self) -> &HermesProtocolHandler {
        unsafe { &(**(self.handler as *const Box<HermesProtocolHandler>)) }
    }

    pub fn user_data(&self) -> &UserData {
        unsafe { &(*(self.user_data as *mut UserData)) }
    }

    pub fn destroy(self) {
        let _ = unsafe { Box::from_raw(self.handler as *mut Box<HermesProtocolHandler>) };
    }
}

#[macro_export]
macro_rules! generate_destroy {
    ($c_symbol:ident for $cstruct:ty) => {
        #[no_mangle]
        pub unsafe extern "C" fn $c_symbol(cstruct: *const $cstruct) -> ffi_utils::SNIPS_RESULT {
            let _ = <$cstruct as RawPointerConverter<$cstruct>>::from_raw_pointer(cstruct);
            ffi_utils::SNIPS_RESULT::SNIPS_RESULT_OK
        }
    };
}

#[macro_export]
macro_rules! generate_facade_wrapper {
    (
        $wrapper_name:ident for
        $facade:ty,
        $drop_name:ident,
        $getter_name:ident = handler.
        $getter:ident
    ) => {
        #[repr(C)]
        pub struct $wrapper_name {
            // hides a Box<$facade>, note the 2 levels (raw pointer + box) to be sure we have a thin pointer here
            facade: *const libc::c_void,
            user_data: *mut libc::c_void,
        }

        impl $wrapper_name {
            pub fn from(facade: Box<$facade>, user_data: UserData) -> Self {
                Self {
                    facade: Box::into_raw(Box::new(facade)) as *const libc::c_void,
                    user_data: user_data.into_raw_pointer() as _,
                }
            }

            pub fn extract(&self) -> &$facade {
                unsafe { &(**(self.facade as *const Box<$facade>)) }
            }

            pub fn user_data(&self) -> &UserData {
                unsafe { &(*(self.user_data as *mut UserData)) }
            }
        }

        impl Drop for $wrapper_name {
            fn drop(&mut self) {
                unsafe { Box::from_raw(self.facade as *mut Box<$facade>) };
            }
        }

        $crate::generate_destroy!($drop_name for $wrapper_name);

        #[no_mangle]
        pub extern "C" fn $getter_name(
            handler: *const CProtocolHandler,
            facade: *mut *const $wrapper_name,
        ) -> ffi_utils::SNIPS_RESULT {
            fn fun(
                handler: *const CProtocolHandler,
                facade: *mut *const $wrapper_name,
            ) -> failure::Fallible<()> {
                let pointer = $wrapper_name::into_raw_pointer($wrapper_name::from(unsafe {
                    let handler = (*handler).extract();
                    (*handler).$getter()
                }, unsafe { (*handler).user_data().duplicate() }));
                unsafe { *facade = pointer };
                Ok(())
            }

            ffi_utils::wrap!(fun(handler, facade))
        }
    };
}

#[macro_export]
macro_rules! generate_facade_publish {
    ($c_symbol:ident = $facade:ty:$method:ident($( + $qualifier_name:ident : $qualifier:ty as $qualifier_raw:ty,)* $arg:ty)) => {
        #[no_mangle]
        pub extern "C" fn $c_symbol(facade : *const $facade, $($qualifier_name : *const $qualifier_raw,)* message : *const $arg) -> ffi_utils::SNIPS_RESULT {
            fn fun(facade : *const $facade, $($qualifier_name : *const $qualifier_raw,)* message : *const $arg) -> failure::Fallible<()> {
                let message = convert(message)?;
                unsafe {(*facade).extract().$method($(<$qualifier as RawBorrow<$qualifier_raw>>::raw_borrow($qualifier_name)?.as_rust()?,)* message)}
            }

            ffi_utils::wrap!(fun(facade, $($qualifier_name,)* message))
        }
    };
    ($c_symbol:ident = $facade:ty:$method:ident($( + $qualifier_name:ident : $qualifier:ty as $qualifier_raw:ty,)*)) => {
        #[no_mangle]
        pub extern "C" fn $c_symbol(facade : *const $facade, $($qualifier_name : *const $qualifier_raw,)*) -> ffi_utils::SNIPS_RESULT {
            fn fun(facade : *const $facade, $($qualifier_name : *const $qualifier_raw,)*) -> failure::Fallible<()> {
                unsafe {(*facade).extract().$method($(<$qualifier as RawBorrow<$qualifier_raw>>::raw_borrow($qualifier_name)?.as_rust()?,)*)}
            }

            ffi_utils::wrap!(fun(facade, $($qualifier_name,)*))
        }
    };

}

#[macro_export]
macro_rules! generate_facade_subscribe {
    ($c_symbol:ident = $facade:ty:$method:ident($( $filter_name:ident : $filter:ty as $filter_raw:ty,)* | $arg:ty | )) => {
        #[no_mangle]
        pub extern "C" fn $c_symbol(facade: *const $facade, $($filter_name : *const $filter_raw,)* handler: Option<unsafe extern "C" fn(*const $arg, *mut libc::c_void)>) -> ffi_utils::SNIPS_RESULT {
            fn fun(facade: *const $facade, $($filter_name : *const $filter_raw,)* handler: Option<unsafe extern "C" fn(*const $arg, *mut libc::c_void)>) -> failure::Fallible<()> {
                let user_data = unsafe { (*facade).user_data().duplicate() };
                let callback = ptr_to_callback(handler, user_data)?;
                unsafe { (*facade).extract().$method($(<$filter as RawBorrow<$filter_raw>>::raw_borrow($filter_name)?.as_rust()?,)* callback) }
            }

            ffi_utils::wrap!(fun(facade, $($filter_name,)* handler))
        }
    };
}

#[macro_export]
macro_rules! generate_hermes_c_symbols {
    () => {

    fn convert<T, U: AsRust<T>>(raw: *const U) -> failure::Fallible<T> {
        unsafe { (*raw).as_rust() }
    }

    fn ptr_to_callback<T, U>(
        ptr: Option<unsafe extern "C" fn(*const U, *mut libc::c_void)>,
        user_data: UserData) -> Fallible<hermes::Callback<T>>
        where
            T: Clone + Sync,
            U: CReprOf<T> + Sync + 'static {
        if let Some(ptr) = ptr {
            Ok(hermes::Callback::new(move |payload: &T| {
                let param = Box::into_raw(Box::new(U::c_repr_of(payload.clone()).unwrap()));
                unsafe { ptr(param, user_data.0) }
            }))
        } else {
            Err(failure::format_err!("null pointer"))
        }
    }

    #[no_mangle]
    pub extern "C" fn hermes_enable_debug_logs() -> ffi_utils::SNIPS_RESULT {
        ffi_utils::wrap!($crate::init_debug_logs())
    }

    #[cfg(feature="full_bindings")]
    $crate::generate_facade_wrapper!(CHotwordFacade for hermes::HotwordFacade,
                                     hermes_drop_hotword_facade,
                                     hermes_protocol_handler_hotword_facade = handler.hotword);
    #[cfg(feature="full_bindings")]
    $crate::generate_facade_subscribe!(hermes_hotword_subscribe_detected = CHotwordFacade:subscribe_detected(hotword_id: std::ffi::CStr as libc::c_char, |CHotwordDetectedMessage|));
    #[cfg(feature="full_bindings")]
    $crate::generate_facade_subscribe!(hermes_hotword_subscribe_all_detected = CHotwordFacade:subscribe_all_detected(|CHotwordDetectedMessage|));

    #[cfg(feature="full_bindings")]
    $crate::generate_facade_wrapper!(CHotwordBackendFacade for hermes::HotwordBackendFacade,
                                     hermes_drop_hotword_backend_facade,
                                     hermes_protocol_handler_hotword_backend_facade = handler.hotword_backend);
    #[cfg(feature="full_bindings")]
    generate_facade_publish!(hermes_hotword_backend_publish_detected = CHotwordBackendFacade:publish_detected(+hotword_id: std::ffi::CStr as libc::c_char, CHotwordDetectedMessage));

    $crate::generate_facade_wrapper!(CSoundFeedbackFacade for hermes::SoundFeedbackFacade,
                                     hermes_drop_sound_feedback_facade,
                                     hermes_protocol_handler_sound_feedback_facade = handler.sound_feedback);

    generate_facade_publish!(hermes_sound_feedback_publish_toggle_on = CSoundFeedbackFacade:publish_toggle_on(CSiteMessage));
    generate_facade_publish!(hermes_sound_feedback_publish_toggle_off = CSoundFeedbackFacade:publish_toggle_off(CSiteMessage));

    #[cfg(feature="full_bindings")]
    $crate::generate_facade_wrapper!(CSoundFeedbackBackendFacade for hermes::SoundFeedbackBackendFacade,
                                     hermes_drop_sound_feedback_backend_facade,
                                     hermes_protocol_handler_sound_feedback_backend_facade = handler.sound_feedback_backend);

    #[cfg(feature="full_bindings")]
    $crate::generate_facade_wrapper!(CAsrFacade for hermes::AsrFacade,
                                     hermes_drop_asr_facade,
                                     hermes_protocol_handler_asr_facade = handler.asr);
    #[cfg(feature="full_bindings")]
    generate_facade_publish!(hermes_asr_publish_start_listening = CAsrFacade:publish_start_listening(CAsrStartListeningMessage));
    #[cfg(feature="full_bindings")]
    generate_facade_publish!(hermes_asr_publish_stop_listening = CAsrFacade:publish_stop_listening(CSiteMessage));
    #[cfg(feature="full_bindings")]
    $crate::generate_facade_subscribe!(hermes_asr_subscribe_text_captured = CAsrFacade:subscribe_text_captured(|CTextCapturedMessage|));
    #[cfg(feature="full_bindings")]
    $crate::generate_facade_subscribe!(hermes_asr_subscribe_partial_text_captured = CAsrFacade:subscribe_partial_text_captured(|CTextCapturedMessage|));

    #[cfg(feature="full_bindings")]
    $crate::generate_facade_wrapper!(CAsrBackendFacade for hermes::AsrBackendFacade,
                                     hermes_drop_asr_backend_facade,
                                     hermes_protocol_handler_asr_backend_facade = handler.asr_backend);
    #[cfg(feature="full_bindings")]
    $crate::generate_facade_subscribe!(hermes_asr_backend_publish_start_listening = CAsrBackendFacade:subscribe_start_listening(|CAsrStartListeningMessage|));
    #[cfg(feature="full_bindings")]
    $crate::generate_facade_subscribe!(hermes_asr_backend_publish_stop_listening = CAsrBackendFacade:subscribe_stop_listening(|CSiteMessage|));
    #[cfg(feature="full_bindings")]
    generate_facade_publish!(hermes_asr_backend_subscribe_text_captured = CAsrBackendFacade:publish_text_captured(CTextCapturedMessage));
    #[cfg(feature="full_bindings")]
    generate_facade_publish!(hermes_asr_backend_subscribe_partial_text_captured = CAsrBackendFacade:publish_partial_text_captured(CTextCapturedMessage));

    #[cfg(feature="full_bindings")]
    $crate::generate_facade_wrapper!(CTtsFacade for hermes::TtsFacade,
                                     hermes_drop_tts_facade,
                                     hermes_protocol_handler_tts_facade = handler.tts);
    #[cfg(feature="full_bindings")]
    generate_facade_publish!(hermes_tts_publish_say = CTtsFacade:publish_say(CSayMessage));
    #[cfg(feature="full_bindings")]
    $crate::generate_facade_subscribe!(hermes_tts_subscribe_say_finished = CTtsFacade:subscribe_say_finished(|CSayFinishedMessage|));

    #[cfg(feature="full_bindings")]
    $crate::generate_facade_wrapper!(CTtsBackendFacade for hermes::TtsBackendFacade,
                                     hermes_drop_tts_backend_facade,
                                     hermes_protocol_handler_tts_backend_facade = handler.tts_backend);
    #[cfg(feature="full_bindings")]
    $crate::generate_facade_subscribe!(hermes_tts_backend_subscribe_say = CTtsBackendFacade:subscribe_say(|CSayMessage|));
    #[cfg(feature="full_bindings")]
    generate_facade_publish!(hermes_tts_backend_publish_say_finished = CTtsBackendFacade:publish_say_finished(CSayFinishedMessage));

    #[cfg(feature="full_bindings")]
    $crate::generate_facade_wrapper!(CNluFacade for hermes::NluFacade,
                                     hermes_drop_nlu_facade,
                                     hermes_protocol_handler_nlu_facade = handler.nlu);
    #[cfg(feature="full_bindings")]
    generate_facade_publish!(hermes_nlu_publish_query = CNluFacade:publish_query(CNluQueryMessage));
    #[cfg(feature="full_bindings")]
    generate_facade_publish!(hermes_nlu_publish_partial_query = CNluFacade:publish_partial_query(CNluSlotQueryMessage));
    #[cfg(feature="full_bindings")]
    $crate::generate_facade_subscribe!(hermes_nlu_subscribe_slot_parsed = CNluFacade:subscribe_slot_parsed(|CNluSlotMessage|));
    #[cfg(feature="full_bindings")]
    $crate::generate_facade_subscribe!(hermes_nlu_subscribe_intent_parsed = CNluFacade:subscribe_intent_parsed(|CNluIntentMessage|));
    #[cfg(feature="full_bindings")]
    $crate::generate_facade_subscribe!(hermes_nlu_subscribe_intent_not_recognized = CNluFacade:subscribe_intent_not_recognized(|CNluIntentNotRecognizedMessage|));

    #[cfg(feature="full_bindings")]
    $crate::generate_facade_wrapper!(CNluBackendFacade for hermes::NluBackendFacade,
                                     hermes_drop_nlu_backend_facade,
                                     hermes_protocol_handler_nlu_backend_facade = handler.nlu_backend);

    #[cfg(feature="full_bindings")]
    $crate::generate_facade_subscribe!(hermes_nlu_backend_subscribe_query = CNluBackendFacade:subscribe_query(|CNluQueryMessage|));
    #[cfg(feature="full_bindings")]
    $crate::generate_facade_subscribe!(hermes_nlu_backend_subscribe_partial_query = CNluBackendFacade:subscribe_partial_query(|CNluSlotQueryMessage|));
    #[cfg(feature="full_bindings")]
    generate_facade_publish!(hermes_nlu_backend_publish_slot_parsed = CNluBackendFacade:publish_slot_parsed(CNluSlotMessage));
    #[cfg(feature="full_bindings")]
    generate_facade_publish!(hermes_nlu_backend_publish_intent_parsed = CNluBackendFacade:publish_intent_parsed(CNluIntentMessage));
    #[cfg(feature="full_bindings")]
    generate_facade_publish!(hermes_nlu_backend_publish_intent_not_recognized = CNluBackendFacade:publish_intent_not_recognized(CNluIntentNotRecognizedMessage));

    #[cfg(feature="full_bindings")]
    $crate::generate_facade_wrapper!(CAudioServerFacade for hermes::AudioServerFacade,
                                     hermes_drop_audio_server_facade,
                                     hermes_protocol_handler_audio_server_facade = handler.audio_server);

    #[cfg(feature="full_bindings")]
    generate_facade_publish!(hermes_audio_server_publish_play_bytes = CAudioServerFacade:publish_play_bytes(CPlayBytesMessage));
    #[cfg(feature="full_bindings")]
    $crate::generate_facade_subscribe!(hermes_audio_server_subscribe_play_finished = CAudioServerFacade:subscribe_play_finished(site_id: std::ffi::CStr as libc::c_char, |CPlayFinishedMessage|));
    #[cfg(feature="full_bindings")]
    $crate::generate_facade_subscribe!(hermes_audio_server_subscribe_all_play_finished = CAudioServerFacade:subscribe_all_play_finished(|CPlayFinishedMessage|));
    #[cfg(feature="full_bindings")]
    $crate::generate_facade_subscribe!(hermes_audio_server_subscribe_audio_frame = CAudioServerFacade:subscribe_audio_frame(site_id: std::ffi::CStr as libc::c_char, |CAudioFrameMessage|));

    #[cfg(feature="full_bindings")]
    $crate::generate_facade_wrapper!(CAudioServerBackendFacade for hermes::AudioServerBackendFacade,
                                     hermes_drop_audio_server_backend_facade,
                                     hermes_protocol_handler_audio_server_backend_facade = handler.audio_server_backend);
    #[cfg(feature="full_bindings")]
    $crate::generate_facade_subscribe!(hermes_audio_server_backend_subscribe_play_bytes = CAudioServerBackendFacade:subscribe_play_bytes(site_id: std::ffi::CStr as libc::c_char,|CPlayBytesMessage|));
    #[cfg(feature="full_bindings")]
    $crate::generate_facade_subscribe!(hermes_audio_server_backend_subscribe_all_play_bytes = CAudioServerBackendFacade:subscribe_all_play_bytes(|CPlayBytesMessage|));
    #[cfg(feature="full_bindings")]
    generate_facade_publish!(hermes_audio_server_backend_publish_play_finished = CAudioServerBackendFacade:publish_play_finished(CPlayFinishedMessage));
    #[cfg(feature="full_bindings")]
    generate_facade_publish!(hermes_audio_server_backend_publish_audio_frame = CAudioServerBackendFacade:publish_audio_frame(CAudioFrameMessage));

    $crate::generate_facade_wrapper!(CDialogueFacade for hermes::DialogueFacade,
                                     hermes_drop_dialogue_facade,
                                     hermes_protocol_handler_dialogue_facade = handler.dialogue);
    $crate::generate_facade_subscribe!(hermes_dialogue_subscribe_session_queued = CDialogueFacade:subscribe_session_queued(|CSessionQueuedMessage|));
    $crate::generate_facade_subscribe!(hermes_dialogue_subscribe_session_started = CDialogueFacade:subscribe_session_started(|CSessionStartedMessage|));
    $crate::generate_facade_subscribe!(hermes_dialogue_subscribe_intent = CDialogueFacade:subscribe_intent(intent_name: std::ffi::CStr as libc::c_char, |CIntentMessage|));
    $crate::generate_facade_subscribe!(hermes_dialogue_subscribe_intents = CDialogueFacade:subscribe_intents(|CIntentMessage|));
    $crate::generate_facade_subscribe!(hermes_dialogue_subscribe_intent_not_recognized = CDialogueFacade:subscribe_intent_not_recognized(|CIntentNotRecognizedMessage|));
    $crate::generate_facade_subscribe!(hermes_dialogue_subscribe_session_ended = CDialogueFacade:subscribe_session_ended(|CSessionEndedMessage|));
    generate_facade_publish!(hermes_dialogue_publish_start_session = CDialogueFacade:publish_start_session(CStartSessionMessage));
    generate_facade_publish!(hermes_dialogue_publish_continue_session = CDialogueFacade:publish_continue_session(CContinueSessionMessage));
    generate_facade_publish!(hermes_dialogue_publish_end_session = CDialogueFacade:publish_end_session(CEndSessionMessage));

    #[cfg(feature="full_bindings")]
    $crate::generate_facade_wrapper!(CDialogueBackendFacade for hermes::DialogueBackendFacade,
                                     hermes_drop_dialogue_backend_facade,
                                     hermes_protocol_handler_dialogue_backend_facade = handler.dialogue_backend);
    #[cfg(feature="full_bindings")]
    generate_facade_publish!(hermes_dialogue_backend_publish_session_queued = CDialogueBackendFacade:publish_session_queued(CSessionQueuedMessage));
    #[cfg(feature="full_bindings")]
    generate_facade_publish!(hermes_dialogue_backend_publish_session_started = CDialogueBackendFacade:publish_session_started(CSessionStartedMessage));
    #[cfg(feature="full_bindings")]
    generate_facade_publish!(hermes_dialogue_backend_publish_intent = CDialogueBackendFacade:publish_intent(CIntentMessage));
    #[cfg(feature="full_bindings")]
    generate_facade_publish!(hermes_dialogue_backend_publish_intent_not_recognized = CDialogueBackendFacade:publish_intent_not_recognized(CIntentNotRecognizedMessage));
    #[cfg(feature="full_bindings")]
    generate_facade_publish!(hermes_dialogue_backend_publish_session_ended = CDialogueBackendFacade:publish_session_ended(CSessionEndedMessage));
    #[cfg(feature="full_bindings")]
    $crate::generate_facade_subscribe!(hermes_dialogue_backend_subscribe_start_session = CDialogueBackendFacade:subscribe_start_session(|CStartSessionMessage|));
    #[cfg(feature="full_bindings")]
    $crate::generate_facade_subscribe!(hermes_dialogue_backend_subscribe_continue_session = CDialogueBackendFacade:subscribe_continue_session(|CContinueSessionMessage|));
    #[cfg(feature="full_bindings")]
    $crate::generate_facade_subscribe!(hermes_dialogue_backend_subscribe_end_session = CDialogueBackendFacade:subscribe_end_session(|CEndSessionMessage|));

    $crate::generate_facade_wrapper!(CInjectionFacade for hermes::InjectionFacade,
                                     hermes_drop_injection_facade,
                                     hermes_protocol_handler_injection_facade = handler.injection);

    generate_facade_publish!(hermes_injection_publish_injection_request = CInjectionFacade:publish_injection_request(CInjectionRequestMessage));
    generate_facade_publish!(hermes_injection_publish_injection_status_request = CInjectionFacade:publish_injection_status_request());
    $crate::generate_facade_subscribe!(hermes_injection_subscribe_injection_status = CInjectionFacade:subscribe_injection_status(|CInjectionStatusMessage|));

    #[cfg(feature="full_bindings")]
    $crate::generate_destroy!(hermes_drop_site_message for CSiteMessage);
    #[cfg(feature="full_bindings")]
    $crate::generate_destroy!(hermes_drop_hotword_detected_message for CHotwordDetectedMessage);
    #[cfg(feature="full_bindings")]
    $crate::generate_destroy!(hermes_drop_text_captured_message for CTextCapturedMessage);
    #[cfg(feature="full_bindings")]
    $crate::generate_destroy!(hermes_drop_nlu_query_message for CNluQueryMessage);
    #[cfg(feature="full_bindings")]
    $crate::generate_destroy!(hermes_drop_nlu_slot_query_message for CNluSlotQueryMessage);
    #[cfg(feature="full_bindings")]
    $crate::generate_destroy!(hermes_drop_play_bytes_message for CPlayBytesMessage);
    #[cfg(feature="full_bindings")]
    $crate::generate_destroy!(hermes_drop_audio_frame_message for CAudioFrameMessage);
    #[cfg(feature="full_bindings")]
    $crate::generate_destroy!(hermes_drop_play_finished_message for CPlayFinishedMessage);
    #[cfg(feature="full_bindings")]
    $crate::generate_destroy!(hermes_drop_say_message for CSayMessage);
    #[cfg(feature="full_bindings")]
    $crate::generate_destroy!(hermes_drop_say_finished_message for CSayFinishedMessage);
     #[cfg(feature="full_bindings")]
    $crate::generate_destroy!(hermes_drop_nlu_slot_message for CNluSlotMessage);
    #[cfg(feature="full_bindings")]
    $crate::generate_destroy!(hermes_drop_nlu_intent_not_recognized_message for CNluIntentNotRecognizedMessage);
    #[cfg(feature="full_bindings")]
    $crate::generate_destroy!(hermes_drop_nlu_intent_message for CNluIntentMessage);
    $crate::generate_destroy!(hermes_drop_intent_message for CIntentMessage);
    $crate::generate_destroy!(hermes_drop_intent_not_recognized_message for CIntentNotRecognizedMessage);
    #[cfg(feature="full_bindings")]
    $crate::generate_destroy!(hermes_drop_start_session_message for CStartSessionMessage);
    $crate::generate_destroy!(hermes_drop_session_started_message for CSessionStartedMessage);
    $crate::generate_destroy!(hermes_drop_session_queued_message for CSessionQueuedMessage);
    #[cfg(feature="full_bindings")]
    $crate::generate_destroy!(hermes_drop_continue_session_message for CContinueSessionMessage);
    #[cfg(feature="full_bindings")]
    $crate::generate_destroy!(hermes_drop_end_session_message for CEndSessionMessage);
    $crate::generate_destroy!(hermes_drop_session_ended_message for CSessionEndedMessage);
    $crate::generate_destroy!(hermes_drop_version_message for CVersionMessage);
    $crate::generate_destroy!(hermes_drop_error_message for CErrorMessage);
    #[cfg(feature="full_bindings")]
    $crate::generate_destroy!(hermes_drop_injection_request_message for CInjectionRequestMessage);
    $crate::generate_destroy!(hermes_drop_injection_status_message for CInjectionStatusMessage);
    };
}
