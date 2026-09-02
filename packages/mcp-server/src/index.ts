import { McpServer } from '@modelcontextprotocol/sdk/server/mcp.js';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { z } from 'zod';
import { spawn } from 'child_process';
import { join, dirname } from 'path';
import { fileURLToPath } from 'url';

const __dirname = dirname(fileURLToPath(import.meta.url));

// CLI path - the diskripper binary
const DISKRIPPER_BIN = process.env.DISKRIPPER_BIN || 'diskripper';

/**
 * Start the DiskRipper MCP server.
 * Exported for direct testing.
 */
export async function startMCPServer(): Promise<void> {
  const server = new McpServer({
    name: 'diskripper-mcp',
    version: '0.1.0',
  });

  // ─── Tool: list_drives ─────────────────────────────────────────
  server.registerTool(
    'list_drives',
    {
      description: 'List all optical drives (CD, DVD, Blu-ray) connected to the system',
      inputSchema: {},
    },
    async () => {
      try {
        const result = await callCLI('list-drives');
        return {
          content: [{ type: 'text' as const, text: result }],
        };
      } catch (error) {
        return {
          content: [{ type: 'text' as const, text: `Error: ${error}` }],
          isError: true,
        };
      }
    }
  );

  // ─── Tool: drive_info ──────────────────────────────────────────
  server.registerTool(
    'drive_info',
    {
      description: 'Get detailed information about a specific optical drive and its disc',
      inputSchema: {
        drive_id: z.string().describe('The ID of the drive to inspect'),
      },
    },
    async ({ drive_id }) => {
      try {
        const result = await callCLI('info', drive_id);
        return {
          content: [{ type: 'text' as const, text: result }],
        };
      } catch (error) {
        return {
          content: [{ type: 'text' as const, text: `Error: ${error}` }],
          isError: true,
        };
      }
    }
  );

  // ─── Tool: rip_disc ────────────────────────────────────────────
  server.registerTool(
    'rip_disc',
    {
      description: 'Rip an optical disc to an image file (ISO, BIN/CUE, etc.)',
      inputSchema: {
        drive_id: z.string().describe('The ID of the drive to rip from'),
        output_path: z.string().describe('Full path where the image file will be saved'),
        format: z.enum(['iso', 'bin', 'img']).optional().default('iso').describe('Image format'),
        verify: z.boolean().optional().default(true).describe('Verify the image after ripping'),
        eject: z.boolean().optional().default(false).describe('Eject disc after ripping completes'),
      },
    },
    async ({ drive_id, output_path, format, verify, eject }) => {
      try {
        const args = ['rip', '--drive', drive_id, '--output', output_path];
        if (format !== 'iso') args.push('--format', format);
        if (verify) args.push('--verify');
        if (eject) args.push('--eject');
        
        const result = await callCLI(...args);
        return {
          content: [{ type: 'text' as const, text: result }],
        };
      } catch (error) {
        return {
          content: [{ type: 'text' as const, text: `Error: ${error}` }],
          isError: true,
        };
      }
    }
  );

  // ─── Tool: extract_files ───────────────────────────────────────
  server.registerTool(
    'extract_files',
    {
      description: 'Extract files from an optical disc to a directory',
      inputSchema: {
        drive_id: z.string().describe('The ID of the drive to extract from'),
        output_dir: z.string().describe('Directory where files will be extracted'),
      },
    },
    async ({ drive_id, output_dir }) => {
      try {
        const result = await callCLI('extract', '--drive', drive_id, '--output', output_dir);
        return {
          content: [{ type: 'text' as const, text: result }],
        };
      } catch (error) {
        return {
          content: [{ type: 'text' as const, text: `Error: ${error}` }],
          isError: true,
        };
      }
    }
  );

  // ─── Tool: rip_audio_cd ────────────────────────────────────────
  server.registerTool(
    'rip_audio_cd',
    {
      description: 'Rip an audio CD to WAV/FLAC files',
      inputSchema: {
        drive_id: z.string().describe('The ID of the CD drive'),
        output_dir: z.string().describe('Directory where audio files will be saved'),
        track: z.number().optional().describe('Specific track number to rip (omit for all tracks)'),
        format: z.enum(['wav', 'flac']).optional().default('wav').describe('Audio output format'),
      },
    },
    async ({ drive_id, output_dir, track, format }) => {
      try {
        const args = ['audio', '--drive', drive_id, '--output', output_dir];
        if (track !== undefined) args.push('--track', String(track));
        if (format !== 'wav') args.push('--format', format);
        
        const result = await callCLI(...args);
        return {
          content: [{ type: 'text' as const, text: result }],
        };
      } catch (error) {
        return {
          content: [{ type: 'text' as const, text: `Error: ${error}` }],
          isError: true,
        };
      }
    }
  );

  // ─── Tool: verify_image ────────────────────────────────────────
  server.registerTool(
    'verify_image',
    {
      description: 'Verify a disc image against the original disc',
      inputSchema: {
        drive_id: z.string().describe('The ID of the drive with the original disc'),
        image_path: z.string().describe('Path to the image file to verify'),
      },
    },
    async ({ drive_id, image_path }) => {
      try {
        const result = await callCLI('verify', '--drive', drive_id, '--image', image_path);
        return {
          content: [{ type: 'text' as const, text: result }],
        };
      } catch (error) {
        return {
          content: [{ type: 'text' as const, text: `Error: ${error}` }],
          isError: true,
        };
      }
    }
  );

  // ─── Tool: list_jobs ───────────────────────────────────────────
  server.registerTool(
    'list_jobs',
    {
      description: 'List all backup jobs and their current status',
      inputSchema: {},
    },
    async () => {
      try {
        const result = await callCLI('list-jobs');
        return {
          content: [{ type: 'text' as const, text: result }],
        };
      } catch (error) {
        return {
          content: [{ type: 'text' as const, text: `Error: ${error}` }],
          isError: true,
        };
      }
    }
  );

  // ─── Tool: job_status ──────────────────────────────────────────
  server.registerTool(
    'job_status',
    {
      description: 'Get the status of a specific backup job',
      inputSchema: {
        job_id: z.string().describe('The ID of the job to check'),
      },
    },
    async ({ job_id }) => {
      try {
        const result = await callCLI('job-status', job_id);
        return {
          content: [{ type: 'text' as const, text: result }],
        };
      } catch (error) {
        return {
          content: [{ type: 'text' as const, text: `Error: ${error}` }],
          isError: true,
        };
      }
    }
  );

  // ─── Tool: cancel_job ──────────────────────────────────────────
  server.registerTool(
    'cancel_job',
    {
      description: 'Cancel a running backup job',
      inputSchema: {
        job_id: z.string().describe('The ID of the job to cancel'),
      },
    },
    async ({ job_id }) => {
      try {
        const result = await callCLI('cancel-job', job_id);
        return {
          content: [{ type: 'text' as const, text: result }],
        };
      } catch (error) {
        return {
          content: [{ type: 'text' as const, text: `Error: ${error}` }],
          isError: true,
        };
      }
    }
  );

  // ─── Start transport ───────────────────────────────────────────
  const transport = new StdioServerTransport();
  await server.connect(transport);
  console.error('DiskRipper MCP server running on stdio');
}

/**
 * Call the diskripper CLI and return stdout.
 * Throws on non-zero exit code.
 */
async function callCLI(...args: string[]): Promise<string> {
  return new Promise((resolve, reject) => {
    const child = spawn(DISKRIPPER_BIN, args, {
      stdio: ['ignore', 'pipe', 'pipe'],
    });

    let stdout = '';
    let stderr = '';

    child.stdout.on('data', (data) => {
      stdout += data.toString();
    });

    child.stderr.on('data', (data) => {
      stderr += data.toString();
    });

    child.on('close', (code) => {
      if (code === 0) {
        resolve(stdout.trim());
      } else {
        reject(new Error(`diskripper exited with code ${code}: ${stderr.trim()}`));
      }
    });

    child.on('error', (err) => {
      reject(new Error(`Failed to spawn diskripper: ${err.message}`));
    });
  });
}
