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

    Ok(())
}
