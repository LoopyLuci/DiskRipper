# DiskRipper PowerShell helper for raw optical drive access on Windows
param(
    [Parameter(Mandatory=$true)]
    [ValidateSet("Read", "Size", "Detect")]
    [string]$Action,
    
    [Parameter(Mandatory=$true)]
    [string]$DriveLetter,
    
    [long]$Offset = 0,
    
    [int]$BytesToRead = 2048
)

$ErrorActionPreference = 'Stop'

$code = @'
using System;
using System.IO;
using System.Runtime.InteropServices;
using Microsoft.Win32.SafeHandles;

namespace DiskRipper
{
    public static class NativeMethods
    {
        [DllImport("kernel32.dll", SetLastError = true, CharSet = CharSet.Unicode)]
        public static extern SafeFileHandle CreateFile(
            string lpFileName, uint dwDesiredAccess, uint dwShareMode,
            IntPtr lpSecurityAttributes, uint dwCreationDisposition,
            uint dwFlagsAndAttributes, IntPtr hTemplateFile);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool ReadFile(
            SafeFileHandle hFile, byte[] lpBuffer, uint nNumberOfBytesToRead,
            out uint lpNumberOfBytesRead, IntPtr lpOverlapped);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool SetFilePointerEx(
            SafeFileHandle hFile, long liDistanceToMove,
            out long lpNewFilePointer, uint dwMoveMethod);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool DeviceIoControl(
            SafeFileHandle hDevice, uint dwIoControlCode,
            IntPtr lpInBuffer, uint nInBufferSize,
            IntPtr lpOutBuffer, uint nOutBufferSize,
            out uint lpBytesReturned, IntPtr lpOverlapped);

        [DllImport("kernel32.dll", SetLastError = true)]
        [return: MarshalAs(UnmanagedType.Bool)]
        public static extern bool CloseHandle(IntPtr hObject);

        const uint GENERIC_READ = 0x80000000;
        const uint FILE_SHARE_READ = 0x00000001;
        const uint FILE_SHARE_WRITE = 0x00000002;
        const uint OPEN_EXISTING = 3;
        const uint FILE_FLAG_SEQUENTIAL_SCAN = 0x08000000;
        const uint FILE_BEGIN = 0;
        const uint IOCTL_CDROM_READ_TOC = 0x00024000;
        const uint IOCTL_CDROM_RAW_READ = 0x0002403E;
        const uint IOCTL_DISK_GET_LENGTH_INFO = 0x0007405C;

        [StructLayout(LayoutKind.Sequential)]
        public struct GET_LENGTH_INFORMATION
        {
            public long Length;
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct CDROM_TOC
        {
            public ushort Length;
            public byte FirstTrack;
            public byte LastTrack;
            [MarshalAs(UnmanagedType.ByValArray, SizeConst = 100)]
            public byte[] TrackData;
        }

        [StructLayout(LayoutKind.Sequential)]
        public struct RAW_READ_INFO
        {
            public long DiskOffset;
            public uint SectorCount;
            public uint TrackMode;
        }

        public static byte[] ReadSectors(string driveLetter, long offset, int bytesToRead)
        {
            string rawPath = "\\\\.\\" + driveLetter + ":";
            SafeFileHandle handle = CreateFile(rawPath, GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE, IntPtr.Zero,
                OPEN_EXISTING, FILE_FLAG_SEQUENTIAL_SCAN, IntPtr.Zero);
            
            if (handle.IsInvalid)
            {
                int error = Marshal.GetLastWin32Error();
                throw new IOException("Failed to open device (error " + error + ")");
            }

            try
            {
                long newPos;
                if (!SetFilePointerEx(handle, offset, out newPos, FILE_BEGIN))
                {
                    int error = Marshal.GetLastWin32Error();
                    throw new IOException("Failed to seek to " + offset + " (error " + error + ")");
                }

                byte[] buffer = new byte[bytesToRead];
                uint bytesRead;
                if (!ReadFile(handle, buffer, (uint)bytesToRead, out bytesRead, IntPtr.Zero))
                {
                    int error = Marshal.GetLastWin32Error();
                    throw new IOException("Failed to read " + bytesToRead + " bytes (error " + error + ")");
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
                handle.Close();
            }
        }

        public static byte[] ReadRawCDDA(string driveLetter, long offset, int sectorSize, int numSectors)
        {
            string rawPath = "\\\\.\\" + driveLetter + ":";
            SafeFileHandle handle = CreateFile(rawPath, GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE, IntPtr.Zero,
                OPEN_EXISTING, FILE_FLAG_SEQUENTIAL_SCAN, IntPtr.Zero);

            if (handle.IsInvalid)
            {
                int error = Marshal.GetLastWin32Error();
                throw new IOException("Failed to open device (error " + error + ")");
            }

            try
            {
                RAW_READ_INFO info = new RAW_READ_INFO();
                info.DiskOffset = offset;
                info.SectorCount = (uint)numSectors;
                info.TrackMode = 2; // CD-DA

                int bufferSize = numSectors * sectorSize;
                byte[] buffer = new byte[bufferSize];
                uint bytesReturned;

                IntPtr inPtr = Marshal.AllocHGlobal(Marshal.SizeOf(info));
                IntPtr outPtr = Marshal.AllocHGlobal(bufferSize);
                try
                {
                    Marshal.StructureToPtr(info, inPtr, false);
                    if (!DeviceIoControl(handle, IOCTL_CDROM_RAW_READ, inPtr, (uint)Marshal.SizeOf(info), outPtr, (uint)bufferSize, out bytesReturned, IntPtr.Zero))
                    {
                        int error = Marshal.GetLastWin32Error();
                        throw new IOException("DeviceIoControl raw read failed (error " + error + ")");
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
                handle.Close();
            }
        }

        public static long GetDiskSize(string driveLetter)
        {
            string rawPath = "\\\\.\\" + driveLetter + ":";
            SafeFileHandle handle = CreateFile(rawPath, GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE, IntPtr.Zero,
                OPEN_EXISTING, FILE_FLAG_SEQUENTIAL_SCAN, IntPtr.Zero);

            if (handle.IsInvalid)
            {
                throw new IOException("Failed to open device: " + Marshal.GetLastWin32Error());
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
                        throw new IOException("DeviceIoControl failed: " + Marshal.GetLastWin32Error());
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
                handle.Close();
            }
        }

        public static int GetTrackCount(string driveLetter)
        {
            string rawPath = "\\\\.\\" + driveLetter + ":";
            SafeFileHandle handle = CreateFile(rawPath, GENERIC_READ,
                FILE_SHARE_READ | FILE_SHARE_WRITE, IntPtr.Zero,
                OPEN_EXISTING, FILE_FLAG_SEQUENTIAL_SCAN, IntPtr.Zero);

            if (handle.IsInvalid)
            {
                throw new IOException("Failed to open device: " + Marshal.GetLastWin32Error());
            }

            try
            {
                CDROM_TOC toc = new CDROM_TOC();
                toc.TrackData = new byte[100 * 11]; // 100 tracks * 11 bytes each
                uint bytesReturned;
                
                IntPtr ptr = Marshal.AllocHGlobal(Marshal.SizeOf(toc));
                try
                {
                    if (!DeviceIoControl(handle, IOCTL_CDROM_READ_TOC, IntPtr.Zero, 0, ptr, (uint)Marshal.SizeOf(toc), out bytesReturned, IntPtr.Zero))
                    {
                        throw new IOException("DeviceIoControl TOC failed: " + Marshal.GetLastWin32Error());
                    }
                    toc = (CDROM_TOC)Marshal.PtrToStructure(ptr, typeof(CDROM_TOC));
                }
                finally
                {
                    Marshal.FreeHGlobal(ptr);
                }
                
                return toc.LastTrack - toc.FirstTrack + 1;
            }
            finally
            {
                handle.Close();
            }
        }
    }
}
'@

Add-Type -TypeDefinition $code -Language CSharp

switch ($Action) {
    "Read" {
        try {
            $bytes = [DiskRipper.NativeMethods]::ReadSectors($DriveLetter, $Offset, $BytesToRead)
            if ($bytes.Length -eq 0) {
                Write-Output ""
            } else {
                [Convert]::ToBase64String($bytes)
            }
        } catch {
            Write-Error "Read failed: $_"
            exit 1
        }
    }
    "Size" {
        try {
            $size = [DiskRipper.NativeMethods]::GetDiskSize($DriveLetter)
            Write-Output $size
        } catch {
            Write-Error "Size failed: $_"
            exit 1
        }
    }
    "Detect" {
        try {
            $tracks = [DiskRipper.NativeMethods]::GetTrackCount($DriveLetter)
            Write-Output $tracks
        } catch {
            Write-Error "Detect failed: $_"
            exit 1
        }
    }
}
