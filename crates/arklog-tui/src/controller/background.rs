use crate::{ConnectionState, FaultLogState};

use super::ArkLogController;

impl ArkLogController {
    pub fn pump_background_tasks(&mut self) -> Result<(), String> {
        self.pump_background_tasks_with_activity().map(|_| ())
    }

    pub fn pump_background_tasks_with_activity(&mut self) -> Result<bool, String> {
        let mut changed = false;
        let mut first_error = None;
        if let Some(result) = self.stream_stop.poll() {
            changed = true;
            let stream_id = self.stopping_stream.take().unwrap_or_default();
            if let Err(error) = self.pump_log_batches() {
                first_error = Some(error);
            }
            if let Err(error) = self.apply_stop_result(&stream_id, result) {
                first_error.get_or_insert(error);
            }
        }
        if let Some(result) = self.device_refresh.poll() {
            changed = true;
            match result {
                Ok(devices) => {
                    if let Err(error) = self.apply_discovered_devices(devices, true) {
                        first_error.get_or_insert(error);
                    }
                }
                Err(error) => {
                    self.connection_state = ConnectionState::Error(error.clone());
                    self.schedule_next_device_refresh();
                    first_error = Some(error);
                }
            }
        }
        if let Some(result) = self.fault_refresh.poll() {
            let requested_device = self.fault_refresh_device_id.take();
            let selected_device = self.selected_device().map(|device| device.id.as_str());
            if requested_device.as_deref() == selected_device {
                changed = true;
                match result {
                    Ok(result) => self.apply_fault_result(result),
                    Err(error) => {
                        self.fault_state = FaultLogState::Error(error.clone());
                        first_error.get_or_insert(error);
                    }
                }
            }
        }
        match self.poll_stream_health() {
            Ok(stream_changed) => changed |= stream_changed,
            Err(error) => {
                first_error.get_or_insert(error);
            }
        }
        match self.reconcile_stream() {
            Ok(stream_changed) => changed |= stream_changed,
            Err(error) => {
                first_error.get_or_insert(error);
            }
        }
        match self.request_scheduled_device_refresh() {
            Ok(refresh_changed) => changed |= refresh_changed,
            Err(error) => {
                first_error.get_or_insert(error);
            }
        }
        first_error.map_or(Ok(changed), Err)
    }
}
