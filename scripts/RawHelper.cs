// DiskRipper Raw Disc Access Helper for Windows
// Compiles to DiskRipper.RawHelper.exe
using System;
using System.IO;
using System.Runtime.InteropServices;

namespace DiskRipper
{
    class RawHelper
    {
        [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
        static extern IntPtr CreateFile(string lpFileName, uint dwDesiredAccess, uint dwShareMode, IntPtr lpSecurityAttributes, uint dwCreationDisposition, uint dwFlagsAndAttributes, IntPtr hTemplateFile);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        static extern bool ReadFile(IntPtr hFile, byte[] lpBuffer, uint nNumberOfBytesToRead, out uint lpNumberOfBytesRead, IntPtr lpOverlapped);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        static extern bool SetFilePointerEx(IntPtr hFile, long liDistanceToMove, out long lpNewFilePointer, uint dwMoveMethod);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        static extern bool DeviceIoControl(IntPtr hDevice, uint dwIoControlCode, IntPtr lpInBuffer, uint nInBufferSize, IntPtr lpOutBuffer, uint nOutBufferSize, out uint lpBytesReturned, IntPtr lpOverlapped);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        static extern bool CloseHandle(IntPtr hObject);

        const uint GENERIC_READ = 0x80000000;
        const uint FILE_SHARE_READ = 0x00000001;
        const uint FILE_SHARE_WRITE = 0x00000002;
        const uint OPEN_EXISTING = 3;
        const uint FILE_FLAG_SEQUENTIAL_SCAN = 0x08000000;
        const uint FILE_BEGIN = 0;
        const uint IOCTL_CDROM_RAW_READ = 0x0002403E;
        const uint IOCTL_DISK_GET_LENGTH_INFO = 0x0007405C;

        [StructLayout(LayoutKind.Sequential)]
        struct GET_LENGTH_INFORMATION
        {
            public long Length;
        }

        [StructLayout(LayoutKind.Sequential)]
        struct RAW_READ_INFO
        {
            public long DiskOffset;
            public uint SectorCount;
            public uint TrackMode;
        }

        static int Main(string[] args)
        {
            if (args.Length < 2)
            {
                Console.Error.WriteLine("Usage: RawHelper <Action> <DriveLetter> [Offset] [BytesToRead]");
                return 1;
            }

            string action = args[0];
            string driveLetter = args[1];

            try
            {
                switch (action.ToLower())
                {
                    case "size":
                        long size = GetDiskSize(driveLetter);
                        Console.Write(size);
                        return 0;

                    case "read":
                        if (args.Length < 4)
                        {
                            Console.Error.WriteLine("Read requires Offset and BytesToRead");
                            return 1;
                        }
                        long offset = long.Parse(args[2]);
                        int bytesToRead = int.Parse(args[3]);
                        byte[] data = ReadSectors(driveLetter, offset, bytesToRead);
                        Console.Write(Convert.ToBase64String(data));
                        return 0;

                    case "readcdda":
                        if (args.Length < 4)
                        {
                            Console.Error.WriteLine("ReadCDDA requires Offset and BytesToRead");
                            return 1;
                        }
                        long cddaOffset = long.Parse(args[2]);
                        int cddaBytes = int.Parse(args[3]);
                        byte[] cddaData = ReadRawCDDA(driveLetter, cddaOffset, cddaBytes);
                        Console.Write(Convert.ToBase64String(cddaData));
                        return 0;

                    default:
                        Console.Error.WriteLine($"Unknown action: {action}");
                        return 1;
                }
            }
            catch (Exception ex)
            {
                Console.Error.WriteLine($"Error: {ex.Message}");
                return 1;
            }
        }

        static long GetDiskSize(string driveLetter)
        {
            string rawPath = "\\\\.\\" + driveLetter + ":";
            IntPtr handle = CreateFile(rawPath, GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE, IntPtr.Zero, OPEN_EXISTING, FILE_FLAG_SEQUENTIAL_SCAN, IntPtr.Zero);

            if (handle.ToInt32() == -1)
            {
                throw new Exception($"Failed to open device: {Marshal.GetLastWin32Error()}");
            }

            try
            {
                GET_LENGTH_INFORMATION lengthInfo = new GET_LENGTH_INFORMATION();
                uint bytesReturned;

                IntPtr ptr = Marshal.AllocHGlobal(Marshal.SizeOf(lengthInfo));
                try
                {
                    if (!DeviceIoControl(handle, IOCTL_DISK_GET_LENGTH_INFO, IntPtr.Zero, 0, ptr, (uint)Marshal.SizeOf(lengthInfo), out bytesReturned, IntPtr.Zero))
                    {
                        throw new Exception($"DeviceIoControl failed: {Marshal.GetLastWin32Error()}");
                    }
                    lengthInfo = (GET_LENGTH_INFORMATION)Marshal.PtrToStructure(ptr, typeof(GET_LENGTH_INFORMATION));
                }
                finally
                {
                    Marshal.FreeHGlobal(ptr);
                }

                return lengthInfo.Length;
            }
            finally
            {
                CloseHandle(handle);
            }
        }

        static byte[] ReadSectors(string driveLetter, long offset, int bytesToRead)
        {
            string rawPath = "\\\\.\\" + driveLetter + ":";
            IntPtr handle = CreateFile(rawPath, GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE, IntPtr.Zero, OPEN_EXISTING, FILE_FLAG_SEQUENTIAL_SCAN, IntPtr.Zero);

            if (handle.ToInt32() == -1)
            {
                throw new Exception($"Failed to open device: {Marshal.GetLastWin32Error()}");
            }

            try
            {
                long newPos;
                if (!SetFilePointerEx(handle, offset, out newPos, FILE_BEGIN))
                {
                    throw new Exception($"Failed to seek: {Marshal.GetLastWin32Error()}");
                }

                byte[] buffer = new byte[bytesToRead];
                uint bytesRead;
                if (!ReadFile(handle, buffer, (uint)bytesToRead, out bytesRead, IntPtr.Zero))
                {
                    throw new Exception($"Failed to read: {Marshal.GetLastWin32Error()}");
                }

                if (bytesRead == 0)
                {
                    return new byte[0];
                }

                byte[] result = new byte[bytesRead];
                Array.Copy(buffer, result, bytesRead);
                return result;
            }
            finally
            {
                CloseHandle(handle);
            }
        }

        static byte[] ReadRawCDDA(string driveLetter, long offset, int bytesToRead)
        {
            string rawPath = "\\\\.\\" + driveLetter + ":";
            IntPtr handle = CreateFile(rawPath, GENERIC_READ, FILE_SHARE_READ | FILE_SHARE_WRITE, IntPtr.Zero, OPEN_EXISTING, FILE_FLAG_SEQUENTIAL_SCAN, IntPtr.Zero);

            if (handle.ToInt32() == -1)
            {
                throw new Exception($"Failed to open device: {Marshal.GetLastWin32Error()}");
            }

            try
            {
                RAW_READ_INFO info = new RAW_READ_INFO();
                info.DiskOffset = offset;
                info.SectorCount = 1;
                info.TrackMode = 2; // CD-DA

                int bufferSize = bytesToRead;
                byte[] buffer = new byte[bufferSize];
                uint bytesReturned;

                IntPtr inPtr = Marshal.AllocHGlobal(Marshal.SizeOf(info));
                IntPtr outPtr = Marshal.AllocHGlobal(bufferSize);
                try
                {
                    Marshal.StructureToPtr(info, inPtr, false);
                    if (!DeviceIoControl(handle, IOCTL_CDROM_RAW_READ, inPtr, (uint)Marshal.SizeOf(info), outPtr, (uint)bufferSize, out bytesReturned, IntPtr.Zero))
                    {
                        throw new Exception($"DeviceIoControl raw read failed: {Marshal.GetLastWin32Error()}");
                    }
                    Marshal.Copy(outPtr, buffer, 0, (int)bytesReturned);
                }
                finally
                {
                    Marshal.FreeHGlobal(inPtr);
                    Marshal.FreeHGlobal(outPtr);
                }

                byte[] result = new byte[bytesReturned];
                Array.Copy(buffer, result, bytesReturned);
                return result;
            }
            finally
            {
                CloseHandle(handle);
            }
        }
    }
}
