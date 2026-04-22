import { Page } from '@playwright/test';

export interface CapturedError {
  message: string;
  stack?: string;
  timestamp: Date;
  type: 'console' | 'page' | 'network';
}

export class ErrorCapture {
  private errors: CapturedError[] = [];

  constructor(private page: Page) {
    this.setupListeners();
  }

  private setupListeners() {
    // Capture console errors
    this.page.on('console', (msg) => {
      if (msg.type() === 'error') {
        this.errors.push({
          message: msg.text(),
          timestamp: new Date(),
          type: 'console',
        });
      }
    });

    // Capture page errors
    this.page.on('pageerror', (error) => {
      this.errors.push({
        message: error.message,
        stack: error.stack,
        timestamp: new Date(),
        type: 'page',
      });
    });

    // Capture failed network requests
    this.page.on('requestfailed', (request) => {
      this.errors.push({
        message: `Request failed: ${request.url()} - ${request.failure()?.errorText}`,
        timestamp: new Date(),
        type: 'network',
      });
    });
  }

  getErrors(): CapturedError[] {
    return this.errors;
  }

  clearErrors() {
    this.errors = [];
  }

  hasErrors(): boolean {
    return this.errors.length > 0;
  }

  printErrors() {
    if (this.errors.length === 0) {
      console.log('No errors captured');
      return;
    }

    console.log('\n=== Captured Errors ===');
    this.errors.forEach((error, index) => {
      console.log(`\n[${index + 1}] ${error.type.toUpperCase()} Error at ${error.timestamp.toISOString()}`);
      console.log(`Message: ${error.message}`);
      if (error.stack) {
        console.log(`Stack: ${error.stack}`);
      }
    });
    console.log('======================\n');
  }
}
