$ErrorActionPreference = 'Stop'

$code = @'
using System;
using System.IO;
using System.Runtime.InteropServices;

namespace DR
{
    public class Raw
    {
        [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
        public static extern IntPtr CreateFile(string lpFileName, uint dwDesiredAccess, uint dwShareMode, IntPtr lpSecurityAttributes, uint dwCreationDisposition, uint dwFlagsAndAttributes, IntPtr hTemplateFile);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool ReadFile(IntPtr hFile, byte[] lpBuffer, uint nNumberOfBytesToRead, out uint lpNumberOfBytesRead, IntPtr lpOverlapped);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool SetFilePointerEx(IntPtr hFile, long liDistanceToMove, out long lpNewFilePointer, uint dwMoveMethod);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool DeviceIoControl(IntPtr hDevice, uint dwIoControlCode, IntPtr lpInBuffer, uint nInBufferSize, IntPtr lpOutBuffer, uint nOutBufferSize, out uint lpBytesReturned, IntPtr lpOverlapped);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool CloseHandle(IntPtr hObject);

        const uint GENERIC_READ = 0x80000000;
        const uint FILE_SHARE_READ = 0x00000001;
        const uint FILE_SHARE_WRITE = 0x00000002;
        const uint OPEN_EXISTING = 3;
        const uint FILE_FLAG_SEQUENTIAL_SCAN = 0x08000000;
        const uint FILE_BEGIN = 0;
        const uint IOCTL_CDROM_RAW_READ = 0x0002403E;
        const uint IOCTL_DISK_GET_LENGTH_INFO = 0x0007405C;
        const uint FSCTL_LOCK_VOLUME = 0x00090018;
        const uint FSCTL_DISMOUNT_VOLUME = 0x00090020;
        const uint FSCTL_UNLOCK_VOLUME = 0x0009002C;

        [StructLayout(LayoutKind.Sequential)]
        public class GET_LENGTH_INFORMATION { public long Length; }

        [StructLayout(LayoutKind.Sequential)]
        public class RAW_READ_INFO
        {
            public long DiskOffset;
            public uint SectorCount;
            public uint TrackMode;
        }

        public static byte[] ReadSectors(string driveLetter, long offset, int bytesToRead)
        {
            string rawPath = "\\\\.\\" + driveLetter + ":";
            IntPtr handle = CreateFile(rawPath, GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE, IntPtr.Zero, OPEN_EXISTING, FILE_FLAG_SEQUENTIAL_SCAN, IntPtr.Zero);

            if (handle.ToInt32() == -1) throw new Exception("Failed to open: " + Marshal.GetLastWin32Error());

            try
            {
                // Try to lock the volume for raw access
                uint bytesReturned;
                DeviceIoControl(handle, FSCTL_DISMOUNT_VOLUME, IntPtr.Zero, 0, IntPtr.Zero, 0, out bytesReturned, IntPtr.Zero);
                DeviceIoControl(handle, FSCTL_LOCK_VOLUME, IntPtr.Zero, 0, IntPtr.Zero, 0, out bytesReturned, IntPtr.Zero);

                long newPos;
                if (!SetFilePointerEx(handle, offset, out newPos, FILE_BEGIN)) throw new Exception("Failed to seek: " + Marshal.GetLastWin32Error());

                byte[] buffer = new byte[bytesToRead];
                uint bytesRead;
                if (!ReadFile(handle, buffer, (uint)bytesToRead, out bytesRead, IntPtr.Zero)) throw new Exception("Failed to read: " + Marshal.GetLastWin32Error());

                if (bytesRead == 0) return new byte[0];
                byte[] result = new byte[bytesRead];
                Array.Copy(buffer, result, bytesRead);
                return result;
            }
            finally { CloseHandle(handle); }
        }

        public static byte[] ReadCDDA(string driveLetter, long offset, int numSectors)
        {
            string rawPath = "\\\\.\\" + driveLetter + ":";
            IntPtr handle = CreateFile(rawPath, GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE, IntPtr.Zero, OPEN_EXISTING, FILE_FLAG_SEQUENTIAL_SCAN, IntPtr.Zero);

            if (handle.ToInt32() == -1) throw new Exception("Failed to open: " + Marshal.GetLastWin32Error());

            try
            {
                RAW_READ_INFO info = new RAW_READ_INFO();
                info.DiskOffset = offset;
                info.SectorCount = (uint)numSectors;
                info.TrackMode = 2; // CD-DA

                int bufferSize = numSectors * 2352;
                byte[] buffer = new byte[bufferSize];
                uint bytesReturned;

                IntPtr inPtr = Marshal.AllocHGlobal(Marshal.SizeOf(info));
                IntPtr outPtr = Marshal.AllocHGlobal(bufferSize);
                try
                {
                    Marshal.StructureToPtr(info, inPtr, false);
                    if (!DeviceIoControl(handle, IOCTL_CDROM_RAW_READ, inPtr, (uint)Marshal.SizeOf(info), outPtr, (uint)bufferSize, out bytesReturned, IntPtr.Zero))
                        throw new Exception("DeviceIoControl raw read failed: " + Marshal.GetLastWin32Error());
                    Marshal.Copy(outPtr, buffer, 0, (int)bytesReturned);
                }
                finally { Marshal.FreeHGlobal(inPtr); Marshal.FreeHGlobal(outPtr); }

                byte[] result = new byte[bytesReturned];
                Array.Copy(buffer, result, bytesReturned);
                return result;
            }
            finally { CloseHandle(handle); }
        }

        public static long GetDiskSize(string driveLetter)
        {
            string rawPath = "\\\\.\\" + driveLetter + ":";
            IntPtr handle = CreateFile(rawPath, GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE, IntPtr.Zero, OPEN_EXISTING, FILE_FLAG_SEQUENTIAL_SCAN, IntPtr.Zero);
            if (handle.ToInt32() == -1) throw new Exception("Failed to open: " + Marshal.GetLastWin32Error());
            try
            {
                GET_LENGTH_INFORMATION info = new GET_LENGTH_INFORMATION();
                uint bytesReturned;
                IntPtr ptr = Marshal.AllocHGlobal(Marshal.SizeOf(info));
                try
                {
                    if (!DeviceIoControl(handle, IOCTL_DISK_GET_LENGTH_INFO, IntPtr.Zero, 0, ptr, (uint)Marshal.SizeOf(info), out bytesReturned, IntPtr.Zero))
                        throw new Exception("DeviceIoControl failed: " + Marshal.GetLastWin32Error());
                    info = (GET_LENGTH_INFORMATION)Marshal.PtrToStructure(ptr, typeof(GET_LENGTH_INFORMATION));
                }
                finally { Marshal.FreeHGlobal(ptr); }
                return info.Length;
            }
            finally { CloseHandle(handle); }
        }
    }
}
'@

Add-Type -TypeDefinition $code -Language CSharp

$action = $args[0]
$driveLetter = $args[1]

switch ($action) {
    "size" { [DR.Raw]::GetSize($driveLetter) }
    "read" { [Convert]::ToBase64String([DR.Raw]::ReadSectors($driveLetter, [long]$args[2], [int]$args[3])) }
    "readcdda" { [Convert]::ToBase64String([DR.Raw]::ReadCDDA($driveLetter, [long]$args[2], [int]$args[3])) }
}
