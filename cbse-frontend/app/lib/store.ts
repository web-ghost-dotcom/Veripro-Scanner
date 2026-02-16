'use client';

import { createContext, useContext } from 'react';

// Types
export interface ProjectFile {
    id: string;
    name: string;
    path: string;
    content: string;
    type: 'contract' | 'spec' | 'other';
}

export interface Project {
    id: string;
    name: string;
    createdAt: string;
    updatedAt: string;
    status: 'idle' | 'verifying' | 'passed' | 'failed';
    source: 'upload' | 'github' | 'paste';
    githubUrl?: string;
    githubBranch?: string;
    files: ProjectFile[];
    lastVerificationId?: string;
}

export interface VerificationResult {
    id: string;
    projectId?: string;
    contractName: string;
    status: 'verified' | 'failed' | 'pending';
    timestamp: string;
    bytecodeHash?: string;
    specHash?: string;
    signature?: string;
    message?: string;
    properties: {
        name: string;
        status: 'passed' | 'failed';
    }[];
    attestation?: {
        prover_address: string;
        result_hash: string;
        signature: string;
        // eslint-disable-next-line @typescript-eslint/no-explicit-any
        payload?: any;
    };
    // On-chain attestation data
    onchainTxHash?: string;
    onchainAttested?: boolean;
}

export interface UserSettings {
    autoAttest: boolean;
    emailNotifications: boolean;
    email: string;
    verificationTimeout: number;
    maxWorkers: number;
    githubConnected: boolean;
    githubUsername?: string;
}

// Storage keys
const PROJECTS_KEY = 'veripro_projects';
const RESULTS_KEY = 'veripro_results';
const SETTINGS_KEY = 'veripro_settings';

// Default settings
const DEFAULT_SETTINGS: UserSettings = {
    autoAttest: false,
    emailNotifications: true,
    email: '',
    verificationTimeout: 300,
    maxWorkers: 4,
    githubConnected: false,
};

// Storage helpers
function getFromStorage<T>(key: string, defaultValue: T): T {
    if (typeof window === 'undefined') return defaultValue;
    try {
        const stored = localStorage.getItem(key);
        return stored ? JSON.parse(stored) : defaultValue;
    } catch {
        return defaultValue;
    }
}

function setToStorage<T>(key: string, value: T): void {
    if (typeof window === 'undefined') return;
    try {
        localStorage.setItem(key, JSON.stringify(value));
    } catch (e) {
        console.error('Failed to save to localStorage:', e);
    }
}

// Project operations
export function getProjects(): Project[] {
    return getFromStorage<Project[]>(PROJECTS_KEY, []);
}

export function getProject(id: string): Project | undefined {
    const projects = getProjects();
    return projects.find(p => p.id === id);
}

export function createProject(data: {
    name: string;
    source: 'upload' | 'github' | 'paste';
    githubUrl?: string;
    githubBranch?: string;
    files?: ProjectFile[];
}): Project {
    const projects = getProjects();
    const newProject: Project = {
        id: `proj_${Date.now()}`,
        name: data.name,
        createdAt: new Date().toISOString(),
        updatedAt: new Date().toISOString(),
        status: 'idle',
        source: data.source,
        githubUrl: data.githubUrl,
        githubBranch: data.githubBranch,
        files: data.files || [],
    };
    projects.unshift(newProject);
    setToStorage(PROJECTS_KEY, projects);
    return newProject;
}

export function updateProject(id: string, updates: Partial<Project>): Project | undefined {
    const projects = getProjects();
    const index = projects.findIndex(p => p.id === id);
    if (index === -1) return undefined;

    projects[index] = {
        ...projects[index],
        ...updates,
        updatedAt: new Date().toISOString(),
    };
    setToStorage(PROJECTS_KEY, projects);
    return projects[index];
}

export function deleteProject(id: string): boolean {
    const projects = getProjects();
    const filtered = projects.filter(p => p.id !== id);
    if (filtered.length === projects.length) return false;
    setToStorage(PROJECTS_KEY, filtered);
    return true;
}

export function addFileToProject(projectId: string, file: Omit<ProjectFile, 'id'>): ProjectFile | undefined {
    const project = getProject(projectId);
    if (!project) return undefined;

    const newFile: ProjectFile = {
        ...file,
        id: `file_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`,
    };

    project.files.push(newFile);
    updateProject(projectId, { files: project.files });
    return newFile;
}

export function updateFileInProject(projectId: string, fileId: string, content: string): boolean {
    const project = getProject(projectId);
    if (!project) return false;

    const file = project.files.find(f => f.id === fileId);
    if (!file) return false;

    file.content = content;
    updateProject(projectId, { files: project.files });
    return true;
}

export function deleteFileFromProject(projectId: string, fileId: string): boolean {
    const project = getProject(projectId);
    if (!project) return false;

    const filteredFiles = project.files.filter(f => f.id !== fileId);
    if (filteredFiles.length === project.files.length) return false;

    updateProject(projectId, { files: filteredFiles });
    return true;
}

// Verification result operations
export function getResults(): VerificationResult[] {
    return getFromStorage<VerificationResult[]>(RESULTS_KEY, []);
}

export function getResult(id: string): VerificationResult | undefined {
    const results = getResults();
    return results.find(r => r.id === id);
}

export function getResultsForProject(projectId: string): VerificationResult[] {
    const results = getResults();
    return results.filter(r => r.projectId === projectId);
}

export function addResult(result: Omit<VerificationResult, 'id' | 'timestamp'>): VerificationResult {
    const results = getResults();
    const newResult: VerificationResult = {
        ...result,
        id: `result_${Date.now()}`,
        timestamp: new Date().toISOString(),
    };
    results.unshift(newResult);
    setToStorage(RESULTS_KEY, results);

    // Update project status if linked
    if (result.projectId) {
        updateProject(result.projectId, {
            status: result.status === 'verified' ? 'passed' : result.status === 'failed' ? 'failed' : 'idle',
            lastVerificationId: newResult.id,
        });
    }

    return newResult;
}

export function deleteResult(id: string): boolean {
    const results = getResults();
    const filtered = results.filter(r => r.id !== id);
    if (filtered.length === results.length) return false;
    setToStorage(RESULTS_KEY, filtered);
    return true;
}

export function clearAllResults(): void {
    setToStorage(RESULTS_KEY, []);
}

export function updateResult(id: string, updates: Partial<VerificationResult>): VerificationResult | undefined {
    const results = getResults();
    const index = results.findIndex(r => r.id === id);
    if (index === -1) return undefined;

    results[index] = { ...results[index], ...updates };
    setToStorage(RESULTS_KEY, results);
    return results[index];
}

// Settings operations
export function getSettings(): UserSettings {
    return getFromStorage<UserSettings>(SETTINGS_KEY, DEFAULT_SETTINGS);
}

export function updateSettings(updates: Partial<UserSettings>): UserSettings {
    const current = getSettings();
    const updated = { ...current, ...updates };
    setToStorage(SETTINGS_KEY, updated);
    return updated;
}

// Generate unique ID
export function generateId(prefix: string = 'id'): string {
    return `${prefix}_${Date.now()}_${Math.random().toString(36).slice(2, 9)}`;
}

// Parse file type from name
export function getFileType(filename: string): 'contract' | 'spec' | 'other' {
    if (filename.endsWith('.spec.sol') || filename.includes('.t.sol') || filename.includes('Test')) {
        return 'spec';
    }
    if (filename.endsWith('.sol')) {
        return 'contract';
    }
    return 'other';
}
