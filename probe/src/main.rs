use starship_battery::{Manager, State};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let manager = Manager::new()?;
    let batteries: Vec<_> = manager.batteries()?.collect();

    if batteries.is_empty() {
        println!("No batteries found.");
        return Ok(());
    }

    println!("Found {} battery/batteries", batteries.len());
    println!();

    for (i, maybe_battery) in batteries.into_iter().enumerate() {
        let battery = maybe_battery?;
        println!("Battery #{}", i);
        println!("  vendor:           {:?}", battery.vendor());
        println!("  model:            {:?}", battery.model());
        println!("  serial:           {:?}", battery.serial_number());
        println!("  technology:       {:?}", battery.technology());
        println!("  state:            {:?}", battery.state());
        println!("  state of charge:  {:.1}%", battery.state_of_charge().value * 100.0);
        println!("  state of health:  {:.1}%", battery.state_of_health().value * 100.0);
        println!("  energy:           {:?}", battery.energy());
        println!("  energy full:      {:?}", battery.energy_full());
        println!("  energy full des.: {:?}", battery.energy_full_design());
        println!("  energy rate:      {:?}", battery.energy_rate());
        println!("  voltage:          {:?}", battery.voltage());
        println!("  temperature:      {:?}", battery.temperature());
        println!("  cycle count:      {:?}", battery.cycle_count());
        println!("  time to full:     {:?}", battery.time_to_full());
        println!("  time to empty:    {:?}", battery.time_to_empty());

        match battery.state() {
            State::Charging => println!("  → currently charging"),
            State::Discharging => println!("  → currently discharging"),
            State::Full => println!("  → fully charged"),
            State::Empty => println!("  → empty"),
            _ => println!("  → state unknown"),
        }
    }

    #[cfg(target_os = "macos")]
    {
        println!();
        println!("=== macOS extras (batty-mac-extras) ===");
        match batty_mac_extras::read() {
            Ok(extras) => {
                println!("  pack lot code:    {:?}", extras.pack_lot_code);
                println!("  pcb lot code:     {:?}", extras.pcb_lot_code);
                println!("  firmware version: {:?}", extras.firmware_version);
                println!("  hardware rev:     {:?}", extras.hardware_revision);
                println!("  cell revision:    {:?}", extras.cell_revision);
                println!("  first use:        {:?}", extras.first_use);
                println!("  technology:       {}", extras.technology);
                println!("  design capacity:  {:?} mAh", extras.design_capacity_mah);
                println!("  full charge cap:  {:?} mAh", extras.nominal_charge_capacity_mah);
                println!("  operating time:   {:?} hours", extras.total_operating_time_hours);
                println!("  max capacity:     {:?} %", extras.max_capacity_percent);
                println!("  vendor (lookup):  {:?}", extras.vendor);
                println!("  health metric:    {:?}", extras.battery_health_metric);
                println!("  max temp ever:    {:?} °C", extras.lifetime_max_temperature_c);
                println!("  peak charge:      {:?} mA", extras.lifetime_max_charge_current_ma);
                println!("  peak voltage:     {:?} mV", extras.lifetime_max_pack_voltage_mv);
                println!("  disconnects:      {:?}", extras.system_disconnect_count);
            }
            Err(e) => println!("  (error: {})", e),
        }
    }

    Ok(())
}
