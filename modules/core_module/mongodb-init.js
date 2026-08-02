const database = db.getSiblingDB("vibea3");

// Optional per-cabinet overrides. Core services do not require a machine record.
database.machines.updateOne(
  { _id: "00010203040506070809" },
  {
    $setOnInsert: {
      eacoin_enabled: true,
      maintenance: false,
      facility: {
        id: "1",
        country: "JP",
        region: "JP-13",
        name: "Vibea3 Arcade",
        country_name: "Japan",
        country_jname: "日本",
        region_name: "Tokyo",
        region_jname: "東京都",
        port: 5700,
        latitude: 35689509,
        longitude: 139691640,
        calendar_year: 2026,
        holidays: []
      }
    }
  },
  { upsert: true }
);

database.cards.updateOne(
  { _id: "E004010000000000" },
  {
    $setOnInsert: {
      user_id: "2200000000000000",
      pin: "1234",
      active: true,
      banned: false,
      eacoin_enabled: true,
      balance: 10000,
      auto_charge_enabled: false,
      auto_charge_threshold: 0,
      auto_charge_amount: 0
    }
  },
  { upsert: true }
);

database.users.updateOne(
  { _id: "2200000000000000" },
  {
    $setOnInsert: { created_at: new Date() },
    $set: { updated_at: new Date() }
  },
  { upsert: true }
);
