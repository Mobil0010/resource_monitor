using System;
using System.Collections.Generic;
using System.Globalization;
using System.Linq;
using System.Management;
using LibreHardwareMonitor.Hardware;

internal static class Program
{
    private sealed class Reading
    {
        public HardwareType Type;
        public string Hardware;
        public string Sensor;
        public float Value;
    }

    private static IEnumerable<ISensor> Sensors(IHardware hardware)
    {
        hardware.Update();
        foreach (var sensor in hardware.Sensors)
            yield return sensor;
        foreach (var child in hardware.SubHardware)
            foreach (var sensor in Sensors(child))
                yield return sensor;
    }

    private static void Main()
    {
        var computer = new Computer
        {
            IsCpuEnabled = true,
            IsMemoryEnabled = true,
            IsStorageEnabled = true,
            IsGpuEnabled = true,
            IsMotherboardEnabled = true
        };
        try
        {
            computer.Open();
            var values = new List<Reading>();
            foreach (var hardware in computer.Hardware)
            {
                foreach (var sensor in Sensors(hardware))
                {
                    if (sensor.SensorType == SensorType.Temperature && sensor.Value.HasValue &&
                        sensor.Value.Value > 0 && sensor.Value.Value <= 150)
                    {
                        values.Add(new Reading
                        {
                            Type = hardware.HardwareType,
                            Hardware = hardware.Name,
                            Sensor = sensor.Name,
                            Value = sensor.Value.Value
                        });
                    }
                }
            }

            var cpu = values.Where(v => v.Type == HardwareType.Cpu)
                .OrderByDescending(v => CpuSensorPriority(v.Sensor))
                .ThenByDescending(v => v.Value).FirstOrDefault();
            var memory = values.Where(v => v.Type == HardwareType.Memory ||
                Contains(v.Sensor, "DIMM", "DRAM", "Memory"))
                .OrderByDescending(v => v.Value).FirstOrDefault();
            var storage = values.Where(v => v.Type == HardwareType.Storage).ToList();
            var systemDriveModel = SystemDriveModel();
            var disk = storage.Where(v => !string.IsNullOrEmpty(systemDriveModel) &&
                    (v.Hardware.IndexOf(systemDriveModel, StringComparison.OrdinalIgnoreCase) >= 0 ||
                     systemDriveModel.IndexOf(v.Hardware, StringComparison.OrdinalIgnoreCase) >= 0))
                .OrderByDescending(v => DiskSensorPriority(v.Sensor))
                .ThenBy(v => v.Value).FirstOrDefault();
            if (disk == null && storage.Select(v => v.Hardware).Distinct().Count() == 1)
                disk = storage.OrderByDescending(v => DiskSensorPriority(v.Sensor))
                    .ThenBy(v => v.Value).FirstOrDefault();

            Print("CPU_TEMPERATURE", cpu);
            Print("MEMORY_TEMPERATURE", memory);
            Print("DISK_TEMPERATURE", disk);
        }
        finally
        {
            computer.Close();
        }
    }

    private static int CpuSensorPriority(string name)
    {
        if (Contains(name, "Package", "Tctl", "Tdie")) return 3;
        if (Contains(name, "Core Average", "CPU")) return 2;
        return 1;
    }

    private static int DiskSensorPriority(string name)
    {
        if (string.Equals(name, "Temperature", StringComparison.OrdinalIgnoreCase) ||
            string.Equals(name, "Drive Temperature", StringComparison.OrdinalIgnoreCase)) return 3;
        if (Contains(name, "Composite", "Average")) return 2;
        return 1;
    }

    private static bool Contains(string value, params string[] candidates)
    {
        return candidates.Any(candidate => value.IndexOf(candidate,
            StringComparison.OrdinalIgnoreCase) >= 0);
    }

    private static void Print(string key, Reading reading)
    {
        if (reading != null)
            Console.WriteLine(key + "=" + reading.Value.ToString(CultureInfo.InvariantCulture));
    }

    private static string SystemDriveModel()
    {
        try
        {
            using (var partitions = new ManagementObjectSearcher(
                "ASSOCIATORS OF {Win32_LogicalDisk.DeviceID='C:'} WHERE AssocClass=Win32_LogicalDiskToPartition"))
            {
                foreach (ManagementObject partition in partitions.Get())
                {
                    using (var disks = new ManagementObjectSearcher(
                        "ASSOCIATORS OF {" + partition.Path.RelativePath + "} WHERE AssocClass=Win32_DiskDriveToDiskPartition"))
                    {
                        foreach (ManagementObject disk in disks.Get())
                            return Convert.ToString(disk["Model"]);
                    }
                }
            }
        }
        catch { }
        return null;
    }
}
