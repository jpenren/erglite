import { test, expect } from '@playwright/test';

test.describe('ErgLite UI', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:1420');
    await page.waitForSelector('.shell', { timeout: 10000 });
  });

  test('should load home screen', async ({ page }) => {
    await expect(page.locator('main >> text=Ready to ride?')).toBeVisible();
    await expect(page.locator('main >> text=Structured training with no subscription')).toBeVisible();
    await expect(page.locator('nav >> text=ErgLite').first()).toBeVisible();
  });

  test('should navigate to devices screen', async ({ page }) => {
    await page.click('nav >> text=Devices');
    await expect(page.locator('text=No trainer connected')).toBeVisible();
    await expect(page.locator('text=Connect your trainer')).toBeVisible();
  });

  test('should navigate to library screen', async ({ page }) => {
    await page.click('nav >> text=Library');
    await expect(page.locator('.chip:has-text("By time")').first()).toBeVisible();
    await expect(page.locator('text=Sweet Spot 3×8')).toBeVisible();
  });

  test('should navigate to routes screen', async ({ page }) => {
    await page.click('nav >> text=Routes');
    await expect(page.locator('h1:has-text("Routes")')).toBeVisible();
  });

  test('should navigate to history screen', async ({ page }) => {
    await page.click('nav >> text=History');
    await expect(page.locator('h1:has-text("History")')).toBeVisible();
  });

  test('should navigate to settings', async ({ page }) => {
    await page.click('nav >> text=Settings');
    await expect(page.locator('text=Profile')).toBeVisible();
  });

  test('should have working sidebar navigation', async ({ page }) => {
    const navItems = ['Home', 'Devices', 'Library', 'Routes', 'History', 'Settings'];
    for (const item of navItems) {
      await page.click(`nav >> text=${item}`);
      await page.waitForTimeout(200);
    }
  });

  test('should show quick start free ride on home', async ({ page }) => {
    await expect(page.locator('main >> text=Free ride')).toBeVisible();
    await expect(page.locator('main >> text=Start riding')).toBeVisible();
  });

  test('should show sample workouts on home', async ({ page }) => {
    await expect(page.locator('main >> h3:has-text("Sweet Spot 3×8")')).toBeVisible();
    await expect(page.locator('main >> h3:has-text("VO2max 4×4")')).toBeVisible();
    await expect(page.locator('main >> h3:has-text("Endurance 60")')).toBeVisible();
  });
});

test.describe('ErgLite Library', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:1420');
    await page.waitForSelector('.shell', { timeout: 10000 });
    await page.click('nav >> text=Library');
    await expect(page.locator('.chip:has-text("By time")').first()).toBeVisible();
  });

  test('should show workout cards', async ({ page }) => {
    await expect(page.locator('text=Sweet Spot 3×8')).toBeVisible();
    await expect(page.locator('text=VO2max 4×4')).toBeVisible();
    await expect(page.locator('text=Endurance 60')).toBeVisible();
  });

  test('should have edit and ride buttons on workouts', async ({ page }) => {
    await expect(page.locator('button:has-text("Ride")').first()).toBeVisible();
    await expect(page.locator('button:has-text("Edit")').first()).toBeVisible();
  });

  test('should switch to routes tab', async ({ page }) => {
    // The tab might be labeled differently - try multiple selectors
    const routesTab = page.locator('button:has-text("Routes"), .chip:has-text("By distance"), [role="tab"]:has-text("Routes")').first();
    await routesTab.click();
    await expect(page.locator('h1:has-text("Routes")')).toBeVisible();
  });
});

test.describe('ErgLite Workout Editor', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:1420');
    await page.waitForSelector('.shell', { timeout: 10000 });
    await page.click('nav >> text=Library');
    await page.click('text=New workout');
    await page.waitForTimeout(500);
  });

  test('should open workout editor', async ({ page }) => {
    // Check for editor title
    await expect(page.getByText('Workout editor')).toBeVisible({ timeout: 5000 });
    // Check for block type buttons (the "+" buttons)
    await expect(page.getByRole('button', { name: '+ Warm-up' })).toBeVisible();
    await expect(page.getByRole('button', { name: '+ Steady' })).toBeVisible();
  });

  test('should show block details when clicked', async ({ page }) => {
    await expect(page.getByText('Workout editor')).toBeVisible();
    // Click on first block to expand details
    await page.getByRole('button', { name: /1 Warm-up/ }).first().click();
    await expect(page.getByText('Name')).toBeVisible();
    await expect(page.getByText('Duration (min)')).toBeVisible();
  });
});

test.describe('ErgLite Devices', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:1420');
    await page.waitForSelector('.shell', { timeout: 10000 });
    await page.click('nav >> text=Devices');
  });

  test('should show trainer connection status', async ({ page }) => {
    await expect(page.locator('text=No trainer connected')).toBeVisible();
    await expect(page.locator('text=Connect your trainer')).toBeVisible();
  });
});

test.describe('ErgLite Settings', () => {
  test.beforeEach(async ({ page }) => {
    await page.goto('http://localhost:1420');
    await page.waitForSelector('.shell', { timeout: 10000 });
    await page.click('nav >> text=Settings');
  });

  test('should show profile settings', async ({ page }) => {
    await expect(page.locator('text=Profile')).toBeVisible();
    await expect(page.locator('text=FTP')).toBeVisible();
  });

  test('should show physics settings', async ({ page }) => {
    await expect(page.locator('text=Physics')).toBeVisible();
    await expect(page.locator('text=Rolling resistance')).toBeVisible();
  });

  test('should show language setting', async ({ page }) => {
    await expect(page.locator('text=Language')).toBeVisible();
    await expect(page.getByRole('button', { name: 'English' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Español' })).toBeVisible();
  });

  test('should show theme setting', async ({ page }) => {
    await expect(page.locator('text=Theme')).toBeVisible();
    await expect(page.getByRole('button', { name: 'Dark' })).toBeVisible();
    await expect(page.getByRole('button', { name: 'Light' })).toBeVisible();
  });

  test('should show data info', async ({ page }) => {
    // Use the heading specifically
    await expect(page.getByRole('heading', { name: 'Data' })).toBeVisible();
    await expect(page.locator('text=local SQLite database')).toBeVisible();
  });
});